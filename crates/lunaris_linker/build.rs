use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cargo_toml_path = Path::new(&manifest_dir).join("Cargo.toml");

    let content = fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");
    
    let mut code = String::new();
    let mut in_dependencies = false;

    for line in content.lines() {
        let line = line.trim();
        
        if line.starts_with("[dependencies]") {
            in_dependencies = true;
            continue;
        } else if line.starts_with('[') {
            in_dependencies = false;
        }

        if in_dependencies && !line.is_empty() && !line.starts_with('#') {
            if let Some(idx) = line.find('=') {
                let dep_name = line[..idx].trim();
                // Sanitize dependency name (replace - with _)
                let crate_name = dep_name.replace("-", "_");
                // Handle quoted keys if any (though unlikely for simple deps)
                let crate_name = crate_name.trim_matches('"').trim_matches('\'');
                
                if !crate_name.is_empty() {
                    code.push_str(&format!("#[allow(unused_imports)]\npub use {} as _;\n", crate_name));
                }
            }
        }
    }

    fs::write(Path::new(&out_dir).join("linking.rs"), code).unwrap();

    // Re-run if Cargo.toml changes
    println!("cargo:rerun-if-changed=Cargo.toml");
}
