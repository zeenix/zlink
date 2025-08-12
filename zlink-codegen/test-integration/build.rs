use std::{env, fs, path::PathBuf};

fn main() {
    // Get the manifest directory.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Process all IDL files.
    let idl_files = ["test.idl", "calc.idl", "storage.idl"];

    // Read all IDL file contents first (they need to stay alive for Interface lifetimes).
    let mut contents = Vec::new();
    for idl_file in &idl_files {
        let idl_path = PathBuf::from(&manifest_dir).join(idl_file);

        // Tell cargo to rerun if the IDL file changes.
        println!("cargo:rerun-if-changed={}", idl_path.display());

        let content = fs::read_to_string(&idl_path)
            .unwrap_or_else(|_| panic!("Failed to read IDL file: {}", idl_path.display()));

        contents.push(content);
    }

    // Parse all interfaces.
    let mut interfaces = Vec::new();
    for content in &contents {
        let interface: zlink::idl::Interface = content
            .as_str()
            .try_into()
            .expect("Failed to parse IDL file");

        interfaces.push(interface);
    }

    // Generate code for all interfaces.
    let generated_code =
        zlink_codegen::generate_interfaces(&interfaces).expect("Failed to generate code");

    // Format the generated code if rustfmt is available.
    let formatted_code = zlink_codegen::format_code(&generated_code).unwrap_or(generated_code);

    // Write generated code to OUT_DIR.
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir).join("generated.rs");
    fs::write(&out_path, formatted_code).expect("Failed to write generated code");
}
