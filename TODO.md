# TODO

* zlink: Provides all the API but leaves actual transport to external crates.
  * Listener trait (code snippet below)
  * Service trait and Server struct (See: <https://github.com/zeenix/zlink-experiments/blob/main/src/main.rs>)
    * generic over Listener
    * new(listener)
    * run(service)
      * Need to make reading from multiple connections working
        * Maybe create a stream from the reader: <https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=bd788e052f78422a9c95f098e7d27797>
        * Remove `alloc` feature from `futures-utils` & update README, if SelectAll from futures is not used.
    * Logging system (abstraction over tracing & defmt?)
      * Replace all `println!` with logging macros.
    * tests
  * FDs
* zlink-tokio
  * Use <https://docs.rs/async-compat/latest/async_compat/>
* zlink-macros
  * service attribute macro (see below)
    * See if we can instead use a macro_rules macro (see <https://docs.rs/pin-project-lite/latest/src/pin_project_lite/lib.rs.html#3-1766> for inspiration)
    * implements `Service` trait
    * handle multiple replies (not covered in the snippet yet)
    * introspection <https://varlink.org/Service>
    * embedded feature
      * Manual Deserialize impl
      * assume fields in a specific order
      * Drop alloc feature of serde
      * Update README
  * tests
  * Update Service docs: Prefer using `service` macro over a manual implementation.
* zlink-usb
  * USB (using nusb) transport
* zlink-micro
  * embassy_usb-based transport
    * Driver impl needs to be provided by the user (e.g `embassy-usb-synopsys-otg` for STM32).
  * Will need to create a connection concept through multiplexing
    * <https://docs.rs/maitake-sync/latest/maitake_sync/struct.WaitMap.html>
  * Ensure cancelation safety (if needed by Server/Service) is satisfied
* zlink-codegen (generates code from IDL)
* zlink-smol

* zlink
  * Update README if we end up never using alloc directly.
  * More efficient parsing of messages in Connection using winnow
    * <https://github.com/winnow-rs/winnow/tree/main/examples/json>
    * Remove the FIXMEs
  * enums support in serde-json-core: <https://github.com/rust-embedded-community/serde-json-core/issues/94>

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
#[varlink::service]
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
#[serde(rename_all = "snake-case")]
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
