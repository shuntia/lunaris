use std::{
    env,
    fs::{DirBuilder, File, rename},
    io::{self, Read, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    str::FromStr,
};

use cargo_toml::Manifest;
use serde::{Deserialize, Serialize};
use tempfile::env::temp_dir;
use toml_edit::{Formatted, Table, Value};
use tracing::{debug, error, info};
use walkdir::WalkDir;

fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().init();
    let workspace_root = env::current_dir()?;

    if workspace_root.join("Cargo.toml.old").exists() {
        error!("Cargo.toml.old exists! Aborting build. run `just cleanup` beforehand.");
        panic!("Cargo.toml exists.");
    }

    let plugin_dir = env::var("LUNARIS_PLUGIN_PATH").unwrap_or("plugins".into());
    let plugin_dir = PathBuf::from(plugin_dir);

    info!("Probing plugins in: {:?}", plugin_dir);

    let mut crate_paths = Vec::new();
    find_cargo_tomls(&plugin_dir, &mut crate_paths)?;
    let mut plugin_parents = Vec::new();

    for path in crate_paths.iter() {
        plugin_parents.push(match path.strip_prefix(&workspace_root) {
            Ok(rel) => rel.parent().unwrap(),
            Err(_) => path.parent().unwrap(),
        })
    }

    info!("Probed and found plugin paths: {:?}", crate_paths);

    let plugins = gather_plugins(plugin_parents)?;

    debug!("Found plugins: {:?}", plugins);

    let mut root_path = workspace_root.clone();
    root_path.push("Cargo.toml");
    let mut root = String::new();
    File::open(&root_path)
        .unwrap()
        .read_to_string(&mut root)
        .unwrap();
    let mut root = toml_edit::DocumentMut::from_str(&root).unwrap();
    root["workspace"]["members"]
        .as_array_mut()
        .unwrap()
        .extend(plugins.iter().map(|el| el.path.to_str().unwrap()));
    root["workspace"]["default-members"]
        .as_array_mut()
        .unwrap()
        .extend(plugins.iter().map(|el| el.path.to_str().unwrap()));

    plugins.iter().for_each(|el| {
        let mut plug_table = Table::new();
        plug_table.insert(
            "path",
            toml_edit::Item::Value(
                Value::from_str(&format!("\"{}\"", el.path.to_str().unwrap())).unwrap(),
            ),
        );

        root["workspace"]["dependencies"]
            .as_table_mut()
            .unwrap()
            .insert(
                &el.cargo.package.as_ref().unwrap().name,
                toml_edit::Item::Table(plug_table),
            );
    });

    let mut core_toml = String::new();
    File::open(workspace_root.join("lunaris_core/Cargo.toml"))
        .unwrap()
        .read_to_string(&mut core_toml)
        .unwrap();
    let mut core_toml = toml_edit::DocumentMut::from_str(&core_toml).unwrap();
    plugins.iter().for_each(|el| {
        core_toml["dependencies"].as_table_mut().unwrap().insert(
            &el.cargo.package.as_ref().unwrap().name,
            toml_edit::Item::Table({
                let mut table = toml_edit::Table::new();
                table.insert(
                    "workspace",
                    toml_edit::Item::Value(Value::Boolean(Formatted::new(true))),
                );
                table
            }),
        );
    });

    let temp_dir = temp_dir().join("lunaris_build");

    DirBuilder::new().recursive(true).create(&temp_dir).unwrap();

    rename(
        workspace_root.join("Cargo.toml"),
        workspace_root.join("Cargo.toml.old"),
    )
    .unwrap();
    File::create(workspace_root.join("Cargo.toml"))
        .unwrap()
        .write_all(root.to_string().as_bytes())
        .unwrap();

    // generated toml for lunaris_core
    rename(
        workspace_root.join("lunaris_core/Cargo.toml"),
        workspace_root.join("lunaris_core/Cargo.toml.old"),
    )
    .unwrap();
    File::create(workspace_root.join("lunaris_core/Cargo.toml"))
        .unwrap()
        .write_all(core_toml.to_string().as_bytes())
        .unwrap();

    // Plugin data
    File::create(temp_dir.join("plugins.toml"))
        .unwrap()
        .write_all(toml::to_string(&plugins).unwrap().as_bytes())
        .unwrap();

    Ok(())
}

fn find_cargo_tomls(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().map_or(false, |n| n == "Cargo.toml") {
            out.push(path.to_path_buf());
        }
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
#[repr(transparent)]
struct PluginCollection {
    content: Vec<PluginCrate>,
}

impl Deref for PluginCollection {
    type Target = Vec<PluginCrate>;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl DerefMut for PluginCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginCrate {
    path: PathBuf,
    cargo: Manifest,
    plugin: PluginConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    name: String,
    features: Vec<PluginFeature>,
}

#[derive(Serialize, Deserialize, Debug)]
enum PluginFeature {
    Gui,
}

fn gather_plugins(paths: Vec<&Path>) -> Result<PluginCollection, io::Error> {
    let mut plugins = Vec::new();
    for path in paths {
        let manifest = cargo_toml::Manifest::from_path(Path::join(path, "Cargo.toml")).unwrap();
        let mut plugin_str = String::new();
        File::open(Path::join(path, "plugin.toml"))
            .unwrap()
            .read_to_string(&mut plugin_str)
            .unwrap();
        let plugin = toml::from_str(&plugin_str).unwrap();
        plugins.push(PluginCrate {
            path: path.to_path_buf(),
            cargo: manifest,
            plugin,
        });
    }
    Ok(PluginCollection { content: plugins })
}
