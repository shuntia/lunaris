use std::{
    env::var,
    path::{Path, PathBuf},
};

fn main() {
    let out_dir = var("OUT_DIR").unwrap();
    let plugin_dir = var("PLUGIN_DIR").unwrap_or("../plugins".into());
    std::fs::write(
        Path::new(&out_dir).join("linking.rs"),
        link(find_crates(PathBuf::from(&plugin_dir))),
    )
    .unwrap();
}

fn find_crates(dir: PathBuf) -> Vec<String> {
    let mut ret = vec![];
    for i in dir.read_dir().unwrap() {
        for j in i.unwrap().path().read_dir().unwrap() {
            ret.push(
                j.unwrap()
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            )
        }
    }
    ret
}

fn link(v: Vec<String>) -> String {
    let mut builder = String::new();
    for i in v {
        builder.push_str(&format!("pub use {i} as _;"));
    }
    builder
}
