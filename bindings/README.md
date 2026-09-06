# Matrix Rust SDK bindings

In this directory, one can find bindings to the Rust SDK that are
maintained by the owners of the Matrix Rust SDK project.

- [`apple`] or `matrix-rust-components-swift`, Swift bindings of the
  [`client-matrix`] crate via [`client-matrix-ffi`],
- [`client-crypto-ffi`], UniFFI (Kotlin, Swift, Python, Ruby) bindings of
  the [`client-crypto`] crate,
- [`client-matrix-ffi`], UniFFI bindings of the [`client-matrix`] crate.

Worked examples of driving [`client-matrix-ffi`] from Swift and Kotlin, covering
client initialisation, authentication, sending a message and basic room
operations, live in [`examples`].

There are also external bindings in other repositories:

- [`crypto-wasm`], JavaScript / WebAssembly bindings of the
  [`client-crypto`] crate,
- [`crypto-nodejs`], Node.js bindings of the
  [`client-crypto`] crate

[`apple`]: ./apple
[`examples`]: ./examples
[`client-crypto-ffi`]: ./client-crypto-ffi
[`client-crypto`]: ../crates/client-crypto
[`client-matrix-ffi`]: ./client-matrix-ffi
[`client-matrix`]: ../crates/client-matrix

[`crypto-wasm`]: https://github.com/matrix-org/matrix-rust-sdk-crypto-wasm
[`crypto-nodejs`]: https://github.com/matrix-org/matrix-rust-sdk-crypto-nodejs

## Contributing

To contribute read this [guide](./CONTRIBUTING.md).
