//! Integration tests that verify generated code compiles and works.

use std::{path::PathBuf, process::Command};

fn run_test_crate(crate_name: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let test_crate_dir = PathBuf::from(manifest_dir).join(crate_name);

    // Build the test crate.
    let output = Command::new("cargo")
        .args(&["build"])
        .current_dir(&test_crate_dir)
        .output()
        .expect(&format!("Failed to run cargo build for {}", crate_name));

    if !output.status.success() {
        eprintln!(
            "Cargo build stderr for {}: {}",
            crate_name,
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("{} failed to compile", crate_name);
    }

    // Run the test binary.
    let binary_path = test_crate_dir.join("target").join("debug").join(crate_name);
    let run_output = Command::new(&binary_path)
        .output()
        .expect(&format!("Failed to run {} binary", crate_name));

    if !run_output.status.success() {
        eprintln!(
            "Run stderr for {}: {}",
            crate_name,
            String::from_utf8_lossy(&run_output.stderr)
        );
        panic!("{} binary failed to run", crate_name);
    }

    let stdout = String::from_utf8_lossy(&run_output.stdout);
    println!("{} output: {}", crate_name, stdout);
}

#[test]
fn test_code_generation() {
    run_test_crate("test-integration");
}
