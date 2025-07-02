//! End-to-end tests for varlink_service::Proxy trait using systemd services.

#![cfg(all(target_os = "linux", feature = "introspection", feature = "idl-parse"))]

use std::path::Path;
use zlink::{unix, varlink_service::Proxy};

const SYSTEMD_MACHINE_SOCKET: &str = "/run/systemd/machine/io.systemd.Machine";

/// Check if the systemd machine socket exists and is accessible.
fn systemd_machine_available() -> bool {
    Path::new(SYSTEMD_MACHINE_SOCKET).exists()
}

#[tokio::test]
async fn test_get_info_systemd_machine() {
    if !systemd_machine_available() {
        eprintln!("Skipping test: systemd machine socket not available");
        return;
    }

    // Connect to systemd machine service
    let mut conn = unix::connect(SYSTEMD_MACHINE_SOCKET)
        .await
        .expect("Failed to connect to systemd machine socket");

    // Test get_info method
    let result = conn.get_info().await;
    assert!(result.is_ok(), "Connection error: {:?}", result);

    let reply = result.unwrap();
    assert!(reply.is_ok(), "Method error: {:?}", reply);

    let info = reply.unwrap();

    // Verify expected systemd service information
    assert_eq!(info.vendor, "The systemd Project");
    assert!(info.product.contains("systemd"));
    assert_eq!(info.url, "https://systemd.io/");

    // Verify that the expected interfaces are present
    let interfaces: Vec<&str> = info.interfaces.iter().copied().collect();
    assert!(interfaces.contains(&"io.systemd.Machine"));
    assert!(interfaces.contains(&"org.varlink.service"));

    println!("✓ get_info test passed");
    println!("  Vendor: {}", info.vendor);
    println!("  Product: {}", info.product);
    println!("  Version: {}", info.version);
    println!("  URL: {}", info.url);
    println!("  Interfaces: {:?}", interfaces);
}

#[tokio::test]
async fn test_get_interface_description_systemd_machine() {
    if !systemd_machine_available() {
        eprintln!("Skipping test: systemd machine socket not available");
        return;
    }

    // Connect to systemd machine service
    let mut conn = unix::connect(SYSTEMD_MACHINE_SOCKET)
        .await
        .expect("Failed to connect to systemd machine socket");

    // Test get_interface_description method
    let result = conn.get_interface_description("io.systemd.Machine").await;
    assert!(result.is_ok(), "Connection error: {:?}", result);

    let interface = result.unwrap().unwrap();
    let interface = interface.parse().unwrap();

    // Verify interface details
    assert_eq!(interface.name(), "io.systemd.Machine");
    assert!(!interface.is_empty());

    // Check for expected methods
    let methods: Vec<_> = interface.methods().collect();
    let method_names: Vec<_> = methods.iter().map(|m| m.name()).collect();

    assert!(method_names.contains(&"Register"));
    assert!(method_names.contains(&"List"));
    assert!(method_names.contains(&"Terminate"));
    assert!(method_names.contains(&"Kill"));
    assert!(method_names.contains(&"Open"));

    // Check for expected custom types
    let custom_types: Vec<_> = interface.custom_types().collect();
    let type_names: Vec<_> = custom_types.iter().map(|t| t.name()).collect();

    assert!(type_names.contains(&"ProcessId"));
    assert!(type_names.contains(&"Timestamp"));
    assert!(type_names.contains(&"Address"));
    assert!(type_names.contains(&"MachineOpenMode"));

    // Check for expected errors
    let errors: Vec<_> = interface.errors().collect();
    let error_names: Vec<_> = errors.iter().map(|e| e.name()).collect();

    assert!(error_names.contains(&"NoSuchMachine"));
    assert!(error_names.contains(&"MachineExists"));
    assert!(error_names.contains(&"NotSupported"));

    println!("✓ get_interface_description test passed");
    println!("  Interface: {}", interface.name());
    println!("  Methods found: {}", methods.len());
    println!("  Custom types found: {}", custom_types.len());
    println!("  Errors defined: {}", errors.len());
}

#[tokio::test]
async fn test_error_handling_invalid_interface() {
    if !systemd_machine_available() {
        eprintln!("Skipping test: systemd machine socket not available");
        return;
    }

    // Connect to systemd machine service
    let mut conn = unix::connect(SYSTEMD_MACHINE_SOCKET)
        .await
        .expect("Failed to connect to systemd machine socket");

    // Test with invalid interface name
    let result = conn
        .get_interface_description("invalid.interface.name")
        .await;
    assert!(result.is_ok(), "Connection should succeed");

    let reply = result.unwrap();
    assert!(reply.is_err(), "Method should return error");

    let error = reply.unwrap_err();
    // The error should be properly typed as varlink_service::Error
    println!("✓ Error handling test passed");
    println!("  Received expected error: {:?}", error);
}

#[tokio::test]
async fn test_proxy_trait_multiple_calls() {
    if !systemd_machine_available() {
        eprintln!("Skipping test: systemd machine socket not available");
        return;
    }

    let mut conn = unix::connect(SYSTEMD_MACHINE_SOCKET)
        .await
        .expect("Failed to connect to systemd machine socket");

    // Test making multiple calls on the same connection
    let info_result = conn.get_info().await;
    assert!(info_result.is_ok());
    assert!(info_result.unwrap().is_ok());

    let _interface_result = conn
        .get_interface_description("io.systemd.Machine")
        .await
        .unwrap()
        .unwrap();

    println!("✓ Multiple calls test passed");
}
