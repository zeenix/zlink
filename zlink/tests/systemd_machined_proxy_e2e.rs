//! End-to-end tests for varlink_service::Proxy trait using systemd-machined service.

#![cfg(all(feature = "introspection", feature = "idl-parse"))]

mod mock_machined_service;

use std::path::Path;
use tempfile::TempDir;
use tokio::{
    select,
    time::{timeout, Duration},
};
use zlink::{
    unix,
    varlink_service::{self, Proxy},
};

#[tokio::test]
async fn introspect_machined() {
    run_test_with_service(|socket_path| async move {
        // Connect to machine service (real or mock)
        let mut conn = timeout(Duration::from_secs(5), unix::connect(&socket_path))
            .await
            .expect("Connection timeout")
            .expect("Failed to connect to machine socket");

        // Test get_info method
        let info = conn.get_info().await.unwrap().unwrap();

        // Verify expected systemd service information
        assert_eq!(info.vendor, "The systemd Project");
        assert!(info.product.contains("systemd"));
        assert_eq!(info.url, "https://systemd.io/");

        // Verify that the expected interfaces are present
        let interfaces: Vec<&str> = info.interfaces.iter().copied().collect();
        assert!(interfaces.contains(&"io.systemd.Machine"));
        assert!(interfaces.contains(&"org.varlink.service"));

        // Test get_interface_description method
        let interface = conn
            .get_interface_description("io.systemd.Machine")
            .await
            .unwrap()
            .unwrap();
        let interface = interface.parse().unwrap();

        // Verify interface details
        assert_eq!(interface.name(), "io.systemd.Machine");
        assert!(!interface.is_empty());

        // Check for expected methods
        let methods: Vec<_> = interface.methods().collect();
        let method_names: Vec<_> = methods.iter().map(|m| m.name()).collect();

        assert!(method_names.contains(&"Register"));
        assert!(method_names.contains(&"Unregister"));
        assert!(method_names.contains(&"Terminate"));
        assert!(method_names.contains(&"Kill"));
        assert!(method_names.contains(&"List"));
        assert!(method_names.contains(&"Open"));

        // Check for expected custom types
        let custom_types: Vec<_> = interface.custom_types().collect();
        let type_names: Vec<_> = custom_types.iter().map(|t| t.name()).collect();

        assert!(type_names.contains(&"AcquireMetadata"));
        assert!(type_names.contains(&"MachineOpenMode"));
        assert!(type_names.contains(&"ProcessId"));
        assert!(type_names.contains(&"Timestamp"));
        assert!(type_names.contains(&"Address"));

        // Check for expected errors
        let errors: Vec<_> = interface.errors().collect();
        let error_names: Vec<_> = errors.iter().map(|e| e.name()).collect();

        assert!(error_names.contains(&"NoSuchMachine"));
        assert!(error_names.contains(&"MachineExists"));
        assert!(error_names.contains(&"NoPrivateNetworking"));
        assert!(error_names.contains(&"NoOSReleaseInformation"));
        assert!(error_names.contains(&"NoUIDShift"));
        assert!(error_names.contains(&"NotAvailable"));
        assert!(error_names.contains(&"NotSupported"));
        assert!(error_names.contains(&"NoIPC"));

        // Test `org.varlink.service` interface impl.
        let interface = conn
            .get_interface_description("org.varlink.service")
            .await
            .unwrap()
            .unwrap();
        let interface = interface.parse().unwrap();
        assert_eq!(&interface, varlink_service::DESCRIPTION);

        // Test with invalid interface name
        let _ = conn
            .get_interface_description("invalid.interface.name")
            .await
            .expect("Connection should succeed")
            .expect_err("Method should return error");

        Ok(())
    })
    .await
    .unwrap();
}

/// Run test with either real systemd service or mock service.
async fn run_test_with_service<F, Fut>(test_fn: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    if use_real_machined_service() {
        // Use real systemd service
        test_fn(DEFAULT_MACHINED_SOCKET.to_string()).await
    } else {
        // Create unique temporary directory and socket path for this test
        let temp_dir = TempDir::new()?;
        let socket_path = temp_dir.path().join("mock.sock");

        // Setup mock service
        let service = mock_machined_service::MockMachinedService::new();
        let listener = unix::bind(&socket_path)?;
        let server = zlink::Server::new(listener, service);

        // Run server and client concurrently
        select! {
            res = server.run() => res?,
            res = test_fn(socket_path.to_string_lossy().to_string()) => res?,
        }

        Ok(())
    }
}

fn use_real_machined_service() -> bool {
    // Ensure user didn't ask for mock service to be used.
    !std::env::var(MOCK_SERVICE_ENV_VAR).is_ok()
        && // Check if the systemd machine socket exists and is accessible.
        Path::new(DEFAULT_MACHINED_SOCKET).exists()
}

const DEFAULT_MACHINED_SOCKET: &str = "/run/systemd/machine/io.systemd.Machine";
const MOCK_SERVICE_ENV_VAR: &str = "ZLINK_MOCK_SERVICE";
