use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

use toml::value::Table;

const BEGIN_MARK: &str = "# BEGIN AUTO-PLUGINS";
const END_MARK: &str = "# END AUTO-PLUGINS";

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let linker_cargo = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("linker/Cargo.toml"));
    let plugin_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("plugins"));

    if !linker_cargo.is_file() {
        return Err(anyhow::err(format!(
            "linker Cargo.toml not found: {}",
            linker_cargo.display()
        )));
    }
    if !plugin_dir.is_dir() {
        return Err(anyhow::err(format!(
            "plugin dir not found: {}",
            plugin_dir.display()
        )));
    }

    let linker_dir = linker_cargo.parent().unwrap().to_path_buf();
    let plugins = discover_plugins(&plugin_dir)?;
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for cargo_toml in plugins {
        if let Some((name, dir)) = read_package_name(&cargo_toml)? {
            let rel = relative_path(&dir, &linker_dir)?;
            entries.push((name, rel));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut content = fs::read_to_string(&linker_cargo)?;
    let deps_block = render_deps(&entries);

    if content.contains(BEGIN_MARK) && content.contains(END_MARK) {
        // replace block
        let new = replace_block(&content, &deps_block);
        fs::write(&linker_cargo, new)?;
        eprintln!(
            "updated: replaced auto plugin block in {}",
            linker_cargo.display()
        );
        return Ok(());
    }

    // ensure [dependencies] exists; if not, append
    if !content.lines().any(|l| l.trim() == "[dependencies]") {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str("\n[dependencies]\n");
        content.push_str(BEGIN_MARK);
        content.push('\n');
        content.push_str(&deps_block);
        content.push_str(END_MARK);
        content.push('\n');
        fs::write(&linker_cargo, content)?;
        eprintln!("updated: appended [dependencies] with auto plugin block");
        return Ok(());
    }

    // insert after first [dependencies]
    let mut out = String::with_capacity(content.len() + deps_block.len() + 64);
    let mut inserted = false;
    for line in content.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted && line.trim() == "[dependencies]" {
            out.push_str(BEGIN_MARK);
            out.push('\n');
            out.push_str(&deps_block);
            out.push_str(END_MARK);
            out.push('\n');
            inserted = true;
        }
    }
    fs::write(&linker_cargo, out)?;
    eprintln!("updated: inserted auto plugin block after [dependencies]");
    Ok(())
}

fn discover_plugins(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for group in read_dir_sorted(root)? {
        let group_path = group.path();
        if !group_path.is_dir() {
            continue;
        }
        for krate in read_dir_sorted(&group_path)? {
            let kp = krate.path();
            let cargo = kp.join("Cargo.toml");
            if cargo.is_file() {
                out.push(cargo);
            }
        }
    }
    Ok(out)
}

fn read_dir_sorted(p: &Path) -> anyhow::Result<Vec<std::fs::DirEntry>> {
    let mut v: Vec<_> = fs::read_dir(p)?.filter_map(|e| e.ok()).collect();
    v.sort_by_key(|e| e.path());
    Ok(v)
}

fn read_package_name(cargo_toml: &Path) -> anyhow::Result<Option<(String, PathBuf)>> {
    let s = fs::read_to_string(cargo_toml)?;
    let v: Table = toml::from_str(&s)?;
    if let Some(pkg) = v.get("package")
        && let Some(name) = pkg.get("name").and_then(|n| n.as_str())
    {
        let dir = cargo_toml
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        return Ok(Some((name.to_string(), dir)));
    }
    Ok(None)
}

fn replace_block(content: &str, deps_block: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_block = false;
    for line in content.lines() {
        let trimmed = line.trim_end();
        if trimmed == BEGIN_MARK {
            in_block = true;
            out.push_str(BEGIN_MARK);
            out.push('\n');
            out.push_str(deps_block);
            continue;
        }
        if trimmed == END_MARK {
            in_block = false;
            out.push_str(END_MARK);
            out.push('\n');
            continue;
        }
        if !in_block {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn render_deps(entries: &[(String, PathBuf)]) -> String {
    let mut s = String::new();
    for (name, path) in entries {
        let line = format!("{name} = {{ path = \"{}\" }}\n", path.display());
        s.push_str(&line);
    }
    s
}

fn relative_path(path: &Path, base: &Path) -> anyhow::Result<PathBuf> {
    let path = std::fs::canonicalize(path)?;
    let base = std::fs::canonicalize(base)?;
    Ok(diff_paths(&path, &base).unwrap_or_else(|| path.clone()))
}

// Minimal diff_paths implementation to avoid extra deps
fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
    let mut ita = path.components();
    let mut itb = base.components();

    // Determine common prefix
    let mut comps_a: Vec<Component<'_>> = Vec::new();
    let mut comps_b: Vec<Component<'_>> = Vec::new();
    loop {
        match (ita.next(), itb.next()) {
            (Some(a), Some(b)) if a == b => {
                comps_a.push(a);
                comps_b.push(b);
            }
            (ra, rb) => {
                // collect rest
                let mut rest_a: Vec<Component<'_>> = ra.into_iter().collect();
                rest_a.extend(ita);
                let mut rest_b: Vec<Component<'_>> = rb.into_iter().collect();
                rest_b.extend(itb);

                let mut result = PathBuf::new();
                for _ in rest_b {
                    result.push("..");
                }
                for c in rest_a {
                    result.push(c.as_os_str());
                }
                if result.as_os_str().is_empty() {
                    result.push(".");
                }
                return Some(result);
            }
        }
    }
}

// tiny anyhow replacement to keep deps minimal
mod anyhow {
    use std::fmt::{Display, Formatter};
    use std::io;

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug)]
    pub struct Error(String);

    impl Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for Error {}

    pub fn err<T: Into<String>>(msg: T) -> Error {
        Error(msg.into())
    }

    impl From<io::Error> for Error {
        fn from(e: io::Error) -> Self {
            Error(e.to_string())
        }
    }

    impl From<toml::de::Error> for Error {
        fn from(e: toml::de::Error) -> Self {
            Error(e.to_string())
        }
    }
}
