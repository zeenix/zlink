
* zarlink: Provides all the API but leaves actual transport to external crates.
  * Socket trait
  * Connection

    * Generic over Socket
    * low-level API to send/receive messages.
    * features of buffer sizes: 4k, 16k (default), 64k, 1M (highest selected if all enabled)
  * Basic tests (start with `add-tests` branch)
  * nostd (Use #![warn(clippy::std_instead_of_core)] if `std` still a feature)
  * CI to build for both std and embedded

* zarlink-macros
  * service attribute macro (takes a mod, see below)
    * keeps Interface trait objects
      * Similar to zbus but no async-trait use (Use Box directly: hint: heapless also has a Box)
    * takes a Connection instance
    * user drives it
    * introspection https://varlink.org/Service
* zarlink-tokio
  * Use https://docs.rs/async-compat/latest/async_compat/
* zarlink-smol
* zarlink-usb
  * USB (using nusb) transport
* zarlink-micro
  * embassy_usb-based transport
* zarlink-gen (generates code from IDL)

* zarlink
  * More efficient parsing of messages in Connection using winnow
    * https://github.com/winnow-rs/winnow/tree/main/examples/json
    * Remove the FIXMEs
  * enums support in serde-json-core: https://github.com/rust-embedded-community/serde-json-core/issues/94
  * Support client-side API for nostd (e.g Connection::receive_reply)

Maybe later:

* zarlink
  * Revive heapfull crate (heapfull branch)
    * Add heapless proxy feature.
    * alloc and heapless feature (one must be enabled)

---------------------------------------

## Code snippets

### service macro

```rust
#[varlink::service]
pub mod my_service {
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

    // The attribute adds a:
    // * A `Socket` generic.
    // * `connection` field.
    // * `get_connection` method.
    // * `new` method that takes a connection.
    // * `<field-name>` and `set_<field-name>` methods for each field.
    // 
    // It also mangles the field names so they're not used directy by the user implementation.
    #[varlink(interface))]
    struct Ftl {
        // This attribute makes `drive_condition` to be sent out as a reply 
        #[varlink(event)]
        drive_condition: DriveCondition,
        coordinates: 
    }

    #[varlink(interface(name = "org.example.ftl"))]
    impl Ftl {
        async fn monitor(&mut self) -> Result<DriveCondition> {
            Ok(self.drive_condition)
        }
    }
}
```
