# Quinn fork for iroh

Quinn is a pure-rust, async-compatible implementation of the IETF
[QUIC][quic] transport protocol.

This is a fork incorporating some changes for use in iroh.  The aim is
to contribute back any generally useful changes into upstream Quinn,
so it is strongly discouraged to use this fork directly.


## Git branches

The default branch for this repo is the `iroh-0.10.x` branch.  This
branch tracks the `quinn-rs:0.10.x` upstream branch, which is the
release branch for the Quinn 0.10 series of releases.

The `main` branch is kept as an identical copy of `quinn-rs:main`
though is probably behind on commits.
