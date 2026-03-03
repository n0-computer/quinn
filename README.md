# noq

[![Documentation](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/noq/)
[![Crates.io](https://img.shields.io/crates/v/iroh.svg?style=flat-square)](https://crates.io/crates/noq)
[![Chat](https://img.shields.io/discord/1161119546170687619?logo=discord&style=flat-square)](https://discord.com/invite/DpmJgtU7cW)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=flat-square)](LICENSE-APACHE)

General purpose implementation of the [QUIC transport
protocol](https://www.rfc-editor.org/rfc/rfc9000.html) in pure
Rust. Noq is built as an async-friendly API in the `noq` crate on top
of a sans-io protocol library in `noq-proto`.

Noq started out as a fork of the excellent
[Quinn](https://github.com/quinn-rs/quinn) project. The main focus of
development has been towards adding support for more QUIC (draft)
extensions:

- [QUIC Multipath](https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/)
- [QUIC Address Discovery](https://datatracker.ietf.org/doc/draft-ietf-quic-address-discovery/) (QAD)
- [Using QUIC to traverse Nat's](https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/) (QNT)

## Features

- Easy to use futures-based async API.
- Client and server server functionality.
- 0-RTT and 0.5-RTT data support.
- Ordered and unordered stream reads.
- Custom and zero-length connection identifiers.
- Fully pluggable crypto API with a [Rustls] implementation using
  [ring] or [aws-lc-rs] provided by default for convenience.
- Broad platform support, including Linux, Windows, macOS, android,
  iOS and wasm.

[Rustls]: https://github.com/rustls/rustls
[ring]: https://github.com/briansmith/ring
[aws-lc-rs]: https://github.com/aws/aws-lc-rs


## Standards

The noq library aims to be correct implementation of various QUIC
standards:

- Supports the core QUIC specifications:
  - [RFC 8999 - Version-Independent Properties of QUIC].
  - [RFC 9000 - QUIC: A UDP-Based Multiplexed and Secure Transport].
  - [RFC 9001 - Using TLS to Secure QUIC].
  - [RFC 9002 - QUIC Loss Detection and Congestion Control].
- The standardised QUIC extensions:
  - [RFC 9221 - An Unreliable Datagram Extension to QUIC].
  - [RFC 9287 - Greasing the QUIC Bit].
  - [RFC 9368 - Compatible Version Negotiation for QUIC].
- Draft extensions:
  - [qlog: Structured Logging for Network Protocols].
  - [QUIC Multipath].
    - With experimental qlog support.
  - [QUIC Address Discovery] (QAD).
  - [Using QUIC to traverse NATs] (QNT).

[RFC 8999 - Version-Independent Properties of QUIC]: https://www.rfc-editor.org/rfc/rfc8999.html
[RFC 9000 - QUIC: A UDP-Based Multiplexed and Secure Transport]: https://www.rfc-editor.org/rfc/rfc9000.html
[RFC 9001 - Using TL'S to Secure QUIC]: https://www.rfc-editor.org/rfc/rfc9001.html
[RFC 9002 - QUIC Loss Detection and Congestion Control]: https://www.rfc-editor.org/rfc/rfc9002.html
[RFC 9221 - An Unreliable Datagram Extension to QUIC]: https://www.rfc-editor.org/rfc/rfc9221.html
[RFC 9287 - Greasing the QUIC Bit]: https://www.rfc-editor.org/rfc/rfc9287.html
[RFC 9368 - Compatible Version Negotiation for QUIC]: https://www.rfc-editor.org/rfc/rfc9368.html
[RFC 9369 - QUIC Version 2]: https://www.rfc-editor.org/rfc/rfc9369.html
[qlog: Structured Logging for Network Protocols]: https://quicwg.org/qlog/draft-ietf-quic-qlog-main-schema.html
[QUIC Multipath]: https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/
[QUIC Address Discovery]: https://datatracker.ietf.org/doc/draft-ietf-quic-address-discovery/
[Using QUIC to traverse NATs]: https://datatracker.ietf.org/doc/draft-seemann-quic-nat-traversal/.


## Getting started

Examples at https://github.com/n0-computer/noq/blob/main/noq/examples

```
$ cargo run --example server ./
$ cargo run --example client https://localhost:4433/Cargo.toml
```

This launches an HTTP 0.9 server over the QUIC transport on the
loopback address serving the current working directory, with the
client fetching ./Cargo.toml. By default, the server generates a
self-signed certificate and stores it to disk, where the client will
automatically find and trust it.

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

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
