use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    // Get the path to zlink-codegen binary.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(&manifest_dir);
    // Go up two levels: test-integration -> zlink-codegen -> workspace
    let workspace_root = manifest_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // Get the target directory.
    let target_dir = workspace_root.join("target").join("debug");
    let codegen_bin = target_dir.join("zlink-codegen");

    // Build zlink-codegen only if it doesn't exist.
    if !codegen_bin.exists() {
        let status = Command::new("cargo")
            .args(&["build", "-p", "zlink-codegen", "--bin", "zlink-codegen"])
            .current_dir(&workspace_root)
            .status()
            .expect("Failed to build zlink-codegen");

        if !status.success() {
            panic!("Failed to build zlink-codegen");
        }
    }

    // Process all IDL files.
    let idl_files = ["test.idl", "calc.idl", "storage.idl"];

    // Generate code for all IDL files in one command.
    let mut cmd = Command::new(&codegen_bin);
    for idl_file in &idl_files {
        let idl_path = PathBuf::from(&manifest_dir).join(idl_file);
        cmd.arg(&idl_path);
        // Tell cargo to rerun if the IDL file changes.
        println!("cargo:rerun-if-changed={}", idl_path.display());
    }

    let output = cmd.output().expect("Failed to run zlink-codegen");

    if !output.status.success() {
        panic!(
            "zlink-codegen failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated_code =
        String::from_utf8(output.stdout).expect("Generated code is not valid UTF-8");

    // Write generated code to OUT_DIR.
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir).join("generated.rs");
    fs::write(&out_path, generated_code).expect("Failed to write generated code");
}
