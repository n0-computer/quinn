# Iroh Quinn

> This is a fork based on [quinn](https://github.com/quinn-rs/quinn), maintained
> by [n0](https://github.com/n0-computer). Currently published to crates.io under
> `iroh-quinn`.

## Main differences to upstream quinn

- Small API improvements
- Implements additional QUIC extensions
  - Multipath
  - QAD
  - QNT
- Expanded support for QLOG

Quinn is a pure-rust, async-compatible implementation of the IETF
[QUIC][quic] transport protocol.

- Simultaneous client/server operation
- Ordered and unordered stream reads for improved performance
- Works on stable Rust, tested on Linux, macOS and Windows
- Pluggable cryptography, with a standard implementation backed by
  [rustls][rustls] and [*ring*][ring]
- Application-layer datagrams for small, unreliable messages
- Future-based async API
- Minimum supported Rust version of 1.83.0


## License

Copyright 2025 The quinn developers
Copyright 2025 N0, INC.

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
