use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=../../config.toml");

    // Get the output directory where the binary will be placed
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    
    // Construct path to target/debug or target/release
    // OUT_DIR is typically: target/debug/build/backend-xxx/out
    // We need to go to: target/debug or target/release
    let out_path = Path::new(&out_dir);
    let target_dir = out_path
        .ancestors()
        .find(|p| p.ends_with(&profile))
        .expect("Could not find target profile directory");

    // Source config.toml from workspace root
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find workspace root");
    
    let source_config = workspace_root.join("config.toml");
    let dest_config = target_dir.join("config.toml");

    // Copy config.toml if it exists
    if source_config.exists() {
        fs::copy(&source_config, &dest_config)
            .unwrap_or_else(|e| panic!("Failed to copy config.toml: {}", e));
        println!("cargo:warning=Copied config.toml to {:?}", dest_config);
    } else {
        println!("cargo:warning=config.toml not found at {:?}, using default config", source_config);
    }
}

