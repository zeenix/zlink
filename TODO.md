# TODO

* zlink-core
  * Ensure fairness in handling method calls from multiple connections
    * Idea: SelectAll takes the index of the last future that won & polls that one the last
  * IDL <https://varlink.org/Service>
    * `idl` mod
    * Type that describes a type: Interface, Method, Type, Error
    * Trait that gives the IDL name of the type
      * impl for common types
    * `Service`
      * `Info` type with fields of `GetInfo` method
      * impl `GetInfo` method for test case
    * cargo features to allow use of `idl` only
    * IntrospectionProxy
* zlink-macros
  * Provide introspection derives
  * Update `Service` example/test to make use of these
* zlink-macros
  * `proxy` attribute macro (wraps `Proxy`)
    * gated behind (default)`proxy` feature
  * `service` attribute macro (see below)
    * gated behind `service` feature
    * See if we can instead use a macro_rules macro (see <https://docs.rs/pin-project-lite/latest/src/pin_project_lite/lib.rs.html#3-1766> for inspiration)
    * implements `Service` trait
    * handle multiple replies (not covered in the snippet yet)
    * introspection <https://varlink.org/Service>
      * Add required API to `Service` trait first
  * tests
  * Update Service docs: Prefer using `service` macro over a manual implementation.
* zlink-codegen (generates code from IDL)
  * Make use of `zlink_core::idl` module
* zlink-usb
  * USB (using nusb) transport
* zlink-micro
  * embassy_usb-based transport
    * Driver impl needs to be provided by the user (e.g `embassy-usb-synopsys-otg` for STM32).
  * Will need to create a connection concept through multiplexing
    * <https://docs.rs/maitake-sync/latest/maitake_sync/struct.WaitMap.html>
  * Ensure cancelation safety (if needed by Server/Service) is satisfied
* zlink-macros
  * embedded feature
    * Manual Deserialize impl
    * assume fields in a specific order
  * alloc/std feature (default)
    * Make alloc feature of serde optional
* More metadata in Cargo.toml files

* zlink-core
  * Pipelining
  * FDs
  * Graceful shutdown
  * More efficient parsing of messages in Connection using winnow
    * <https://github.com/winnow-rs/winnow/tree/main/examples/json>
    * Remove the FIXMEs
  * enums support in serde-json-core: <https://github.com/rust-embedded-community/serde-json-core/issues/94>
* zlink-smol
* zlink-tokio
  * notified
    * Send out last message on drop
      * builder-pattern setter method to disable this.
    * Split Stream so that we don't require Clone for `Once`

---------------------------------------

## Code snippets

### service macro

```rust
struct Ftl {
    drive_condition: DriveCondition,
    coordinates: Coordinate,
}

// This attribute macro defines a varlink service that can be passed to `Server::run`.
//
// It supports the folowing sub-attributes:
// * `interface`: The interface name. If this is given than all the methods will be prefixed
//   with the interface name. This is useful when the service only offers a single interface.
#[zlink_tokio::service]
impl Ftl {
    #[zlink(interface = "org.varlink.service.ftl")]
    async fn monitor(&mut self) -> Result<DriveCondition> {
        Ok(self.drive_condition)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DriveCondition {
    state: DriveState,
    tylium_level: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveState {
    Idle,
    Spooling,
    Busy,
}

#[derive(Debug, Serialize, Deserialize)]
struct DriveConfiguration {
    speed: i64,
    trajectory: i64,
    duration: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Coordinate {
    longitude: f32,
    latitude: f32,
    distance: i64,
}
```
