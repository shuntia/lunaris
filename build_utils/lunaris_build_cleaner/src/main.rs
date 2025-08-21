use std::{
    env::{self, temp_dir},
    fs::{remove_file, rename},
};

fn main() -> Result<(), std::io::Error> {
    let workspace_root = env::current_dir()?;

    if !workspace_root.join("Cargo.toml.old").exists() {
        panic!("Cannot recover old cargo toml! did you run `just prepare`?")
    }
    remove_file(workspace_root.join("Cargo.toml"))?;
    rename(
        workspace_root.join("Cargo.toml.old"),
        workspace_root.join("Cargo.toml"),
    )?;

    remove_file(workspace_root.join("lunaris_core/Cargo.toml"))?;
    rename(
        workspace_root.join("lunaris_core/Cargo.toml.old"),
        workspace_root.join("lunaris_core/Cargo.toml"),
    )?;

    remove_file(temp_dir().join("lunaris_build/plugins.toml"))?;

    Ok(())
}
