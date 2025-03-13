# zlink

An asynchronous no-std-compatible Varlink Rust crate. It consists for the following subcrates:

* `zlink`: A no-std crate that provides all the core API. It leaves the actual transport to
  other crates.
* `zlink-tokio`: Tranport based on Unix-domain sockets API of `tokio`.
* `zlink-usb` & `zlink-micro`: Together these enables RPC between a (Linux) host and
  microcontrollers through USB. Use the former on the host side and latter on the microcontrollers
  side.

## Why does zlink require a global allocator?

Originally, `zlink` was also intended to be no_alloc as well but due to a series of hurdles, this
idea was abandoned. For example we need to make use of the enum representations in `serde` but [most
enum representations in `serde` require `alloc`][meris].

Still, we make every effort to minimize allocations as much as possible. In fact, we don't do any allocations in `zlink` itself unless `std` feature is enabled.

[meris]: https://github.com/serde-rs/serde-rs.github.io/pull/179
