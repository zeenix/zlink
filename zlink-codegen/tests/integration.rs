//! Integration tests that verify generated code compiles and works.

use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;
use zlink::idl::Interface;
use zlink_codegen::generate_interface;

#[test]
fn test_generated_code_compiles() {
    let idl = r#"
# Test interface with various features
interface org.example.test

type Person (
    name: string,
    age: int,
    email: ?string
)

error NotFound(id: int)
error InvalidInput(message: string)

method Ping(message: string) -> (reply: string)
method GetPerson(id: int) -> (person: Person)
method ListPeople() -> (people: []Person)
method UpdatePerson(person: Person) -> ()
"#;

    let interface = Interface::try_from(idl).unwrap();
    let generated_code = generate_interface(&interface).unwrap();

    // Create a temporary directory for our test project
    let temp_dir = tempdir().unwrap();
    let test_project_dir = temp_dir.path();

    // Create a Cargo.toml for the test project
    let workspace_path = PathBuf::from("/home/zeenix/checkout/zeenix/zlink");
    let cargo_toml = format!(
        r#"
[package]
name = "test-generated"
version = "0.1.0"
edition = "2021"

[dependencies]
zlink = {{ path = "{}/zlink" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["rt", "macros"] }}
anyhow = "1.0"

[[bin]]
name = "test-bin"
path = "src/main.rs"
"#,
        workspace_path.display()
    );

    fs::write(test_project_dir.join("Cargo.toml"), cargo_toml).unwrap();

    // Create src directory
    fs::create_dir(test_project_dir.join("src")).unwrap();

    // Write the generated code to a module
    let generated_module_path = test_project_dir.join("src/generated.rs");
    fs::write(&generated_module_path, &generated_code).unwrap();

    // Create a main.rs that uses the generated code
    let main_rs = r#"
mod generated;

use generated::*;
use zlink::Connection;
use std::collections::HashMap;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Test that we can create instances of generated types
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        email: Some("alice@example.com".to_string()),
    };
    
    // Test that we can create error variants
    let _error1 = TestError::NotFound { id: 42 };
    let _error2 = TestError::InvalidInput { 
        message: "bad input".to_string() 
    };
    
    println!("Generated code compiles and types work!");
    Ok(())
}
"#;

    fs::write(test_project_dir.join("src/main.rs"), main_rs).unwrap();

    // Run cargo build to verify the generated code compiles
    let output = Command::new("cargo")
        .args(&["build", "--bin", "test-bin"])
        .current_dir(test_project_dir)
        .env("CARGO_TARGET_DIR", test_project_dir.join("target"))
        .env("CARGO_HOME", workspace_path.join("target/.cargo"))
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        eprintln!(
            "Cargo build stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!("Generated code:\n{}", generated_code);
        panic!("Generated code failed to compile");
    }

    // Run the binary to ensure it works
    let run_output = Command::new(test_project_dir.join("target/debug/test-bin"))
        .output()
        .expect("Failed to run test binary");

    assert!(run_output.status.success(), "Test binary failed to run");
    assert!(String::from_utf8_lossy(&run_output.stdout).contains("Generated code compiles"));
}

#[test]
fn test_proxy_trait_usage() {
    let idl = r#"
interface org.example.calc

method Add(a: int, b: int) -> (result: int)
method Divide(a: int, b: int) -> (result: float)
"#;

    let interface = Interface::try_from(idl).unwrap();
    let generated_code = generate_interface(&interface).unwrap();

    // Create a test that uses the proxy trait
    let temp_dir = tempdir().unwrap();
    let test_project_dir = temp_dir.path();

    let workspace_path = PathBuf::from("/home/zeenix/checkout/zeenix/zlink");
    let cargo_toml = format!(
        r#"
[package]
name = "test-proxy"
version = "0.1.0"
edition = "2021"

[dependencies]
zlink = {{ path = "{}/zlink" }}
zlink-tokio = {{ path = "{}/zlink-tokio" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["rt", "macros"] }}
anyhow = "1.0"
"#,
        workspace_path.display(),
        workspace_path.display()
    );

    fs::write(test_project_dir.join("Cargo.toml"), cargo_toml).unwrap();
    fs::create_dir(test_project_dir.join("src")).unwrap();
    fs::write(test_project_dir.join("src/generated.rs"), &generated_code).unwrap();

    let main_rs = r#"
mod generated;

use generated::*;
use zlink::Connection;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // This test just verifies that the proxy trait is generated correctly
    // and that the method signatures are as expected
    
    // We can't easily test actual proxy usage without a real connection,
    // but we can at least verify the trait exists and has the right shape
    
    fn assert_calc_proxy<T: CalcProxy>() {}
    
    println!("Proxy trait compiles correctly!");
    Ok(())
}

// The integration test is just checking that the generated code compiles
// We don't need to implement the trait, just verify it exists
"#;

    fs::write(test_project_dir.join("src/main.rs"), main_rs).unwrap();

    let output = Command::new("cargo")
        .args(&["build"])
        .current_dir(test_project_dir)
        .env("CARGO_TARGET_DIR", test_project_dir.join("target"))
        .env(
            "CARGO_HOME",
            "/home/zeenix/checkout/zeenix/zlink/target/.cargo",
        )
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        eprintln!(
            "Cargo build stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!("Generated code:\n{}", generated_code);
        panic!("Proxy trait code failed to compile");
    }
}

#[test]
fn test_service_types() {
    let idl = r#"
interface org.example.storage

type Document (
    id: string,
    title: string,
    content: string,
    tags: []string
)

method Store(doc: Document) -> (id: string)
method Retrieve(id: string) -> (doc: Document)
method Search(query: string, tags: []string) -> (docs: []Document)
"#;

    let interface = Interface::try_from(idl).unwrap();
    let generated_code = generate_interface(&interface).unwrap();

    let temp_dir = tempdir().unwrap();
    let test_project_dir = temp_dir.path();

    let workspace_path = PathBuf::from("/home/zeenix/checkout/zeenix/zlink");
    let cargo_toml = format!(
        r#"
[package]
name = "test-service"
version = "0.1.0"
edition = "2021"

[dependencies]
zlink = {{ path = "{}/zlink" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["rt", "macros"] }}
anyhow = "1.0"
"#,
        workspace_path.display()
    );

    fs::write(test_project_dir.join("Cargo.toml"), cargo_toml).unwrap();
    fs::create_dir(test_project_dir.join("src")).unwrap();
    fs::write(test_project_dir.join("src/generated.rs"), &generated_code).unwrap();

    let main_rs = r#"
mod generated;

use generated::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Test creating and using generated types
    let _doc = Document {
        id: "doc123".to_string(),
        title: "Test Document".to_string(),
        content: "This is test content".to_string(),
        tags: vec!["test".to_string(), "example".to_string()],
    };
    
    // Test that the proxy trait exists and compiles
    fn assert_storage_proxy<T: StorageProxy>() {}
    
    println!("Service types compile and work correctly!");
    Ok(())
}
"#;

    fs::write(test_project_dir.join("src/main.rs"), main_rs).unwrap();

    let output = Command::new("cargo")
        .args(&["build"])
        .current_dir(test_project_dir)
        .env("CARGO_TARGET_DIR", test_project_dir.join("target"))
        .env(
            "CARGO_HOME",
            "/home/zeenix/checkout/zeenix/zlink/target/.cargo",
        )
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        eprintln!(
            "Cargo build stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!("Generated code:\n{}", generated_code);
        panic!("Service types code failed to compile");
    }

    // Run the binary
    let run_output = Command::new(test_project_dir.join("target/debug/test-service"))
        .output()
        .expect("Failed to run test binary");

    assert!(
        run_output.status.success(),
        "Service test binary failed to run"
    );
}
