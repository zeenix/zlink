//! Code generation for Varlink interfaces.

use anyhow::{Context, Result};
use std::{
    fs,
    path::Path,
};
use zlink::idl::Interface;

mod codegen;
pub use codegen::CodeGenerator;

/// Generate Rust code from a Varlink interface.
pub fn generate_interface(interface: &Interface<'_>) -> Result<String> {
    let mut generator = CodeGenerator::new();
    generator.generate_interface(interface, false)?;
    Ok(generator.output())
}

/// Generate Rust code from multiple Varlink interfaces.
pub fn generate_interfaces(interfaces: &[Interface<'_>]) -> Result<String> {
    let mut generator = CodeGenerator::new();

    // Add module-level header for multiple interfaces.
    if interfaces.len() > 1 {
        generator.write_module_header()?;
    }

    for interface in interfaces.iter() {
        // Skip module header for all interfaces when generating multiple.
        let skip_header = interfaces.len() > 1;
        generator.generate_interface(interface, skip_header)?;
    }
    Ok(generator.output())
}

/// Format generated Rust code using rustfmt.
pub fn format_code(code: &str) -> Result<String> {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    let mut child = Command::new("rustfmt")
        .arg("--edition=2021")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn rustfmt")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(code.as_bytes())
            .context("Failed to write to rustfmt stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for rustfmt")?;

    if !output.status.success() {
        // If rustfmt fails, return the original code.
        eprintln!(
            "Warning: rustfmt failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(code.to_string());
    }

    String::from_utf8(output.stdout).context("Failed to parse rustfmt output")
}

/// For future options like type definitions
#[derive(Default)]
pub struct CodegenOptions {}

/// Create source file, can be used in build.rs
pub fn create_source_file<T: AsRef<Path> + ?Sized>(
    input_file: &T,
    rustfmt: bool,
    #[allow(unused_variables)] config: &CodegenOptions,
) {
    let input_path = input_file.as_ref();
    let input_path_no_ext = input_path.with_extension("");
    let new_filename = input_path_no_ext
        .file_name()
        .unwrap_or_else(|| {
            eprintln!("Error: Invalid input path");
            std::process::exit(1)
        })
        .to_str()
        .unwrap_or_else(|| {
            eprintln!("Error: Invalid input path");
            std::process::exit(1)
        })
        .replace('.', "_");
    let output_path = input_path
        .parent()
        .unwrap_or_else(|| {
            eprintln!("Failed to create output_path");
            std::process::exit(1)
        })
        .join(Path::new(&new_filename).with_extension("rs"));

    // Read from input file
    let content = fs::read_to_string(input_path).unwrap_or_else(|_| {
        eprintln!("Failed to read file: {}", input_path.display());
        std::process::exit(1)
    });

    // Parse and generate the interface
    let interface = Interface::try_from(content.as_str()).unwrap_or_else(|_| {
        eprintln!("Failed to parse interface from: {}", input_path.display());
        std::process::exit(1)
    });
    let mut output = generate_interface(&interface).unwrap_or_else(|e| {
        eprintln!(
            "Failed to generate code for interface {}: {e}",
            interface.name()
        );
        std::process::exit(1)
    });

    // Format the code
    if rustfmt {
        output = format_code(&output).unwrap_or_else(|e| {
            eprintln!("Failed to format code: {e}");
            std::process::exit(1)
        });
    }

    // Write output to file.
    fs::write(&output_path, output).unwrap_or_else(|_| {
        eprintln!("Failed to write output file: {}", output_path.display());
        std::process::exit(1)
    });
}
