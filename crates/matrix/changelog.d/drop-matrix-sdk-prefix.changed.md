Every crate lost its `matrix-sdk` prefix: `matrix-sdk` is now `matrix`,
`matrix-sdk-base` is `base`, `matrix-sdk-crypto` is `crypto`, and so on.
`matrix-sdk-common`, `matrix-sdk-state-res`, `matrix-sdk-test*` and
`matrix-sdk-ffi*` became `sdk-common`, `sdk-state-res`, `sdk-test*` and
`matrix-ffi*`, because the bare names are taken or reserved. The SQLite
database file names, the IndexedDB database and object store names, the
store-encryption HKDF info string, the dehydrated-device account data key and
the `rs.matrix-sdk.*` event types are unchanged, so stored data still reads
back.
