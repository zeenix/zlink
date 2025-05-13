# zlink

An asynchronous no-std-compatible Varlink Rust crate. It consists for the following subcrates:

* `zlink-core`: A no-std and no-alloc crate that provides all the core API. It leaves the actual
  transport & high-level API to other crates. This crate is not intended to be used directly.
* `zlink`: The main crate that provides a unified API and will typically be the crate you'd use
  directly. It re-exports API from the appropriate crate(s) depending on the cargo feature(s)
  enabled. This crate also provides high-level macros to each write clients and services.
* `zlink-tokio`: `tokio`-based transport implementations and runtime integration.
* `zlink-usb` & `zlink-micro`: Together these enables RPC between a (Linux) host and
  microcontrollers through USB. The former is targetted for the the host side and latter for the
  microcontrollers side.
