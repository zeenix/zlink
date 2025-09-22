use std::{env, path::PathBuf};

fn main() {
    // Get the manifest directory.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Process all IDL files.
    let idl_files = ["test.idl", "calc.idl", "storage.idl"];

    // Read all IDL file contents first (they need to stay alive for Interface lifetimes).
    for idl_file in &idl_files {
        let idl_path = PathBuf::from(&manifest_dir).join(idl_file);

        zlink_codegen::generate_file(
            idl_path.as_os_str(),
            &zlink_codegen::CodegenOptions {
                rustfmt: true,
            },
        ).expect("Failed to generate code");
    }
}
