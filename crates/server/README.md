# harana-matrix-server

Building blocks for a Matrix homeserver: application service registrations,
server name resolution, state resolution, a store codec and thumbnailing.

Each module is behind a feature of the same name, all enabled by default. See
the [crate documentation](https://docs.rs/harana-matrix-server) for what each
one covers.

The protocol types these are built on live in
[`harana-matrix-common`](https://crates.io/crates/harana-matrix-common).
