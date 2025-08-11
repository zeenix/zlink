use anyhow::{Context, Result};
use clap::Parser;
use heck::ToSnakeCase;
use std::{
    fmt::Write as FmtWrite,
    fs,
    io::{self, Write},
};
use zlink::idl::Interface;
use zlink_codegen::{format_code, generate_interface};

mod cli;
use cli::Args;

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle the case where no command is provided (use files directly).
    let (files, output, multiple_files) = match args.command {
        Some(cli::Command::Generate {
            files,
            output,
            multiple_files,
        }) => (files, output, multiple_files),
        None => (args.files, args.output, args.multiple_files),
    };

    if files.is_empty() {
        eprintln!("Error: No input files specified");
        eprintln!("Usage: zlink-codegen <FILES>... [OPTIONS]");
        std::process::exit(1);
    }

    // Parse all interfaces from input files.
    // We need to keep the file contents alive because Interface borrows from them.
    let mut file_contents = Vec::new();
    let mut interfaces = Vec::new();
    for file_path in &files {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        file_contents.push(content);
    }

    for (i, file_path) in files.iter().enumerate() {
        let interface = Interface::try_from(file_contents[i].as_str())
            .with_context(|| format!("Failed to parse interface from: {}", file_path.display()))?;

        interfaces.push(interface);
    }

    // Generate code based on output options.
    if let Some(output_path) = output {
        // Single output file.
        let mut output = String::new();

        // Write header.
        writeln!(&mut output, "//! Generated code from Varlink IDL files.")?;
        writeln!(&mut output)?;

        for interface in &interfaces {
            let code = generate_interface(interface).with_context(|| {
                format!(
                    "Failed to generate code for interface: {}",
                    interface.name()
                )
            })?;

            writeln!(&mut output, "{}", code)?;
        }

        // Format the code.
        let formatted = format_code(&output)?;

        // Write to file.
        fs::write(&output_path, formatted)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

        println!("Generated code written to {}", output_path.display());
    } else if multiple_files {
        // Multiple output files.
        for interface in &interfaces {
            let code = generate_interface(interface).with_context(|| {
                format!(
                    "Failed to generate code for interface: {}",
                    interface.name()
                )
            })?;

            // Format the code.
            let formatted = format_code(&code)?;

            // Generate output filename from interface name.
            let filename = interface_to_filename(interface.name());
            let output_path = format!("{}.rs", filename);

            // Write to file.
            fs::write(&output_path, formatted)
                .with_context(|| format!("Failed to write output file: {}", output_path))?;

            println!(
                "Generated code for `{}` written to {}",
                interface.name(),
                output_path
            );
        }
    } else {
        // Output to stdout.
        let mut output = String::new();

        if interfaces.len() > 1 {
            writeln!(&mut output, "//! Generated code from Varlink IDL files.")?;
            writeln!(&mut output)?;
        }

        for interface in &interfaces {
            let code = generate_interface(interface).with_context(|| {
                format!(
                    "Failed to generate code for interface: {}",
                    interface.name()
                )
            })?;

            writeln!(&mut output, "{}", code)?;
        }

        // Format the code.
        let formatted = format_code(&output)?;

        // Write to stdout.
        io::stdout().write_all(formatted.as_bytes())?;
    }

    Ok(())
}

fn interface_to_filename(interface_name: &str) -> String {
    // Convert interface name like "org.example.Interface" to "interface".
    interface_name
        .split('.')
        .next_back()
        .unwrap_or(interface_name)
        .to_snake_case()
}
