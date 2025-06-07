# zlink

An asynchronous no-std-compatible Varlink Rust crate. It consists for the following subcrates:

* `zlink-core`: A no-std crate that provides all the core API. It leaves the actual transport &
  high-level API to other crates. This crate is not intended to be used directly.
* `zlink`: The main crate that provides a unified API and will typically be the crate you'd use
  directly. It re-exports API from the appropriate crate(s) depending on the cargo feature(s)
  enabled. This crate also provides high-level macros to each write clients and services.
* `zlink-tokio`: `tokio`-based transport implementations and runtime integration.
* `zlink-usb` & `zlink-micro`: Together these enables RPC between a (Linux) host and
  microcontrollers through USB. The former is targetted for the the host side and latter for the
  microcontrollers side.

## Why does zlink-core require a global allocator?

Originally, `zlink-core` was also intended to be no_alloc as well but due to a series of hurdles,
this idea was abandoned. For example we need to make use of the enum representations in `serde` but
[most enum representations in `serde` require `alloc`][meris].

Still, we make every effort to minimize allocations as much as possible.

[meris]: https://github.com/serde-rs/serde-rs.github.io/pull/179
