<h1 align="center">harana-matrix</h1>

This is a higher velocity fork of [matrix-rusk-sdk](https://github.com/matrix-org/matrix-rust-sdk).

That means that we add features, merge PRs, fix bugs, update dependencies etc at a rapid pace and rely exclusively on automated testing and Claude / Codex to maintain some semblance of quality. We will port features/issues from the official SDK on an ongoing basis but may choose to use our implementation if it makes sense.

## Differences

We already diverge in a number of ways:

* Everything is pluggable. Which means Tokio, Sqlite, Tantivy, TLS etc are all optional.
* [harana-olm](https://github.com/harana/harana-olm) is used as it fixes a number of security issues.

## Bindings

The higher-level crates of the Matrix Rust SDK can be embedded in other environments such as Swift, Kotlin, JavaScript, and Node.js. Check out the [bindings/](./bindings/) directory to learn more about how to integrate the SDK into your language of choice.

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

[Matrix]: https://matrix.org/
[Rust]: https://www.rust-lang.org/
