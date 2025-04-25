# zlink

An asynchronous no-std-compatible Varlink Rust crate. It consists for the following subcrates:

* `zlink-core`: A no-std and no-alloc crate that provides all the core API. It leaves the actual
  transport & high-level API to other crates. This crate is not intended to be used directly.
* `zlink-tokio`: Tranport based on Unix-domain sockets API of `tokio` and high-level API.
* `zlink-usb` & `zlink-micro`: Together these enables RPC between a (Linux) host and
  microcontrollers through USB. Use the former on the host side and latter on the microcontrollers
  side.
