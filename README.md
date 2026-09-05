<h1 align="center">harana-matrix</h1>

This is a higher velocity fork of [matrix-rusk-sdk](https://github.com/matrix-org/matrix-rust-sdk).

That means that we add features, merge PRs, fix bugs, update dependencies etc at a rapid pace and rely exclusively on automated testing and Claude / Codex to maintain some semblance of quality. We will port features/issues from the official SDK on an ongoing basis but may choose to use our implementation if it makes sense.

## Differences

We diverge in a number of ways:

* *Everything* is pluggable so Tokio, Sqlite, Tantivy, TLS etc are all optional.
* [Ruma](https://github.com/ruma/ruma), [Vodozemac](https://github.com/matrix-org/vodozemac), [Tuwunel](https://github.com/matrix-construct/tuwunel) have all been inlined to make the library simpler and improve velocity. 

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

[Matrix]: https://matrix.org/
[Rust]: https://www.rust-lang.org/
