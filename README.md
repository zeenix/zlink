# zarlink

An asynchronous no-std-compatible Varlink Rust crate. It consists for the following subcrates:

* `zarlink-core`: A no-std crate that provides all the core API. It leaves the actual transport to
  other crates.
* `zarlink-tokio`: Tranport based on Unix-domain sockets API of `tokio`.
* `zarlink-usb` & `zarlink-micro`: Together these enables RPC between a (Linux) host and
  microcontrollers through USB. Use the former on the host side and latter on the microcontrollers
  side.
