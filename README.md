<h1 align="center">harana-matrix</h1>

This is a higher velocity fork of [matrix-rusk-sdk](https://github.com/matrix-org/matrix-rust-sdk).

That means that we add features, merge PRs, fix bugs, update dependencies etc at a rapid pace and rely exclusively on automated testing and Claude / Codex to maintain some semblance of quality. We will port features/issues from the official SDK on an ongoing basis but may choose to use our implementation if it makes sense.

## Differences

We diverge in a number of ways:

* *Everything* is pluggable so Tokio, Sqlite, Tantivy, TLS etc are all optional.
* [Ruma](https://github.com/ruma/ruma), [Vodozemac](https://github.com/harana/harana-olm), [Tuwunel](https://github.com/matrix-construct/tuwunel) have all been inlined to make the library simpler and improve velocity.
* There are three crates rather than thirty, one per tier, with a module (and usually a feature) where each of the old crates used to be.

## Crates

| crate | what it is |
| --- | --- |
| [`harana-matrix-client`](./crates/client) | the client SDK: the `Client`, the stores, the crypto state machine, the UI layer |
| [`harana-matrix-common`](./crates/common) | the Matrix protocol types and algorithms, and the Olm ratchets; shared by both sides |
| [`harana-matrix-server`](./crates/server) | homeserver-side building blocks |
| [`harana-matrix-macros`](./crates/macros) | the derive and attribute macros the three above are built on |

See [ARCHITECTURE.md](./ARCHITECTURE.md) for what lives where.

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

[Matrix]: https://matrix.org/
[Rust]: https://www.rust-lang.org/
