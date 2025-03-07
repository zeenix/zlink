# zarlink

An asynchronous no-std-compatible Varlink Rust crate. It consists for the following subcrates:

* `zarlink`: A no-std crate that provides all the core API. It leaves the actual transport to
  other crates.
* `zarlink-tokio`: Tranport based on Unix-domain sockets API of `tokio`.
* `zarlink-usb` & `zarlink-micro`: Together these enables RPC between a (Linux) host and
  microcontrollers through USB. Use the former on the host side and latter on the microcontrollers
  side.

## Why does zarlink require a global allocator?

Originally, `zarlink` was also intended to be no_alloc as well but due to a serious of hurdles, this
idea was abandoned. For example we need to make use of APIs from certain external crates that
require `alloc`:

* [Most enum representations in `serde`][meris].
* `futures_util::future::select_all`.

Still, we make every effort to minimize allocations as much as possible.

[meris]: https://github.com/serde-rs/serde-rs.github.io/pull/179
