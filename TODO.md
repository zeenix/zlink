# TODO

* zlink: Provides all the API but leaves actual transport to external crates.
  * Connection::call_method
  * Listener trait (code snippet below)
  * Service trait and Server struct (code snippet below)
    * generic over Listener
    * new(listener)
    * run(service)
    * handle multiple replies (not covered in the snippet yet)
    * tests
  * FDs
* zlink-tokio
  * Use <https://docs.rs/async-compat/latest/async_compat/>
* zlink-macros
  * service attribute macro (see below)
    * implements `Service` trait
    * handle multiple replies (not covered in the snippet yet)
    * introspection <https://varlink.org/Service>
  * tests
* zlink-smol
* zlink-usb
  * USB (using nusb) transport
* zlink-micro
  * embassy_usb-based transport
  * Will need to create a connection concept through multiplexing
    * <https://docs.rs/maitake-sync/latest/maitake_sync/struct.WaitMap.html>
* zlink-codegen (generates code from IDL)

* zlink
  * Update README if we end up never using alloc directly.
  * More efficient parsing of messages in Connection using winnow
    * <https://github.com/winnow-rs/winnow/tree/main/examples/json>
    * Remove the FIXMEs
  * enums support in serde-json-core: <https://github.com/rust-embedded-community/serde-json-core/issues/94>
  * Support client-side API for nostd (e.g Connection::receive_reply)

---------------------------------------

## Code snippets

### Service

```rust
pub struct Server<L> {
    listener: L,
}

impl<L> Server<L>
where
    L: Listener,
{
    async fn run<Srv>(&mut self, mut service_impl: Srv)
    where
        for<'de, 'ser> Srv: Service<'de, 'ser>,
    {
        let mut connection = self.listener.accept().await;
        loop {
            // Safety: TODO:
            let service_impl = unsafe { &mut *(&mut service_impl as *mut Srv) };
            if let Err(_) = service_impl.handle_next(&mut connection).await {
                break;
            }
        }
    }
}

pub trait Service<'de, 'ser> {
    type MethodCall: Deserialize<'de>;
    type Reply: Serialize;

    fn handle(&'ser mut self, method: Self::MethodCall) -> impl Future<Output = Self::Reply>;

    fn handle_next<Sock>(
        &'ser mut self,
        connection: &'de mut Connection<Sock>,
    ) -> impl Future<Output = Result<(), ()>>
    where
        Sock: Socket,
    {
        async {
            let json = connection.read_json_from_socket().await?;
            let call: Self::MethodCall = serde_json::from_str(json).unwrap();
            let _: Self::Reply = self.handle(call).await;

            Ok(())
        }
    }
}

pub trait Listener {
    type Socket: Socket;

    fn accept(&mut self) -> impl Future<Output = Connection<Self::Socket>>;
}

// Thsi would be a `tokio::net::UnixListener`.
impl Listener for () {
    type Socket = SocketNext;

    async fn accept(&mut self) -> Connection<Self::Socket> {
        Connection {
            socket: SocketNext::GetName,
            buf: [0; 1024],
        }
    }
}

pub trait Socket {
    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<usize, ()>>;
    fn write(&mut self, buf: &[u8]) -> impl Future<Output = Result<usize, ()>>;
}

pub struct Connection<Socket> {
    socket: Socket,
    buf: [u8; 1024],
}

impl<Sock> Connection<Sock>
where
    Sock: Socket,
{
    async fn read_json_from_socket(&mut self) -> Result<&str, ()> {
        let len = self.socket.read(&mut self.buf).await?;
        let json = std::str::from_utf8(&self.buf[..len]).unwrap();
        Ok(json)
    }
}
```

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
