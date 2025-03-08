# TODO

* zarlink: Provides all the API but leaves actual transport to external crates.
  * Connection

    * Generic over Socket
    * low-level API to send/receive messages.
    * features of buffer sizes: 4k, 16k (default), 64k, 1M (highest selected if all enabled)
  * Listener trait
  * Service (code snippet below)
    * generic over Listener
    * new(listener)
  * Basic tests (start with `add-tests` branch)
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
  * Will need to create a connection concept through multiplexing
    * https://docs.rs/maitake-sync/latest/maitake_sync/struct.WaitMap.html
* zarlink-gen (generates code from IDL)

* zarlink
  * Update README if we end up never using alloc directly.
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

### Service

```rust
struct Service<L> {
    listener: L,
}

impl<L> Service<L>
where
    L: Listener,
{
      async fn run<'h, Handler, MethodCall, Reply>(
        &'h mut self,
        mut handler: Handler,
    ) -> Result<(), Error>
    where
        Handler: AsyncFnMut(&'h mut Self, MethodCall) -> Reply,
        MethodCall: Deserialize<'h>,
        Reply: Serialize,
    {
        loop {
            // Receive the next message from the connection.
            let call: MethodCall = serde_json::from_str("{ \"x\": 32 }").unwrap();
            let _: Reply = handler(self, call).await;
            // Send reply on the connection.
        }

        Ok(())
    }
}
```

### service macro

```rust
#[varlink::service]
pub mod my_service {
    // The attribute adds a:
    // * A `Listener` generic.
    // * `new` method that takes a connection.
    // * `<field-name>` and `set_<field-name>` methods for each field.
    #[varlink(interface))]
    struct Ftl {
        drive_condition: DriveCondition,
        coordinates:
    }

    #[varlink(interface(name = "org.example.ftl"))]
    impl Ftl {
        // Special args:
        //
        // * `connection`: Reference to the connection which received the call.
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
}
```
