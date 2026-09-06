Every crate now carries a tier prefix saying who it is for: `common-` for the
Matrix protocol types and shared test helpers, `client-` for the client SDK and
its bindings, `server-` for the homeserver-side building blocks. `matrix` is
now `client-matrix`, `base` is `client-base`, `crypto` is `client-crypto`,
`sdk-common` is `client-common`, `events` is `common-events`, `ruma` is
`common-ruma`, and so on; the vendored `ruma-common` became `common-types`
rather than `common-common`, and the vendored Olm implementation moved from
`crates/harana-olm` to `crates/common-olm` and is no longer published under the
`vodozemac` package name. Re-exported modules keep their own names, so
`common_ruma::events` and `client_common::ruma` are spelled as before. The
SQLite database file names, the IndexedDB database and object store names, the
store-encryption HKDF info string, the dehydrated-device account data key and
the `rs.matrix-sdk.*` event types are unchanged, so stored data still reads
back. The generated FFI artifacts follow the crate names: the libraries are now
`libclient_matrix_ffi` and `libclient_crypto_ffi`.
