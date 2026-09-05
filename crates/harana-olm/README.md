# harana-olm (vendored)

The Olm and Megolm implementation used by this workspace, vendored from
[harana/harana-olm](https://github.com/harana/harana-olm), a fork of
[vodozemac](https://github.com/matrix-org/vodozemac) that carries a number of
security fixes.

The package keeps the name `vodozemac` so that every consumer in the workspace
can go on writing `use vodozemac::...`, which keeps upstream `matrix-rust-sdk`
merges free of conflicts. The directory is named after the fork it comes from.

## Provenance

| Item | Value |
| --- | --- |
| Upstream | <https://github.com/harana/harana-olm> |
| Commit | `062cfc656ccac996faa2e00c7c82a4ff36690c3c` |
| Version | 0.10.0 |

## Re-syncing

`src/` and `afl/*/in/` are copied verbatim, so a re-sync is a straight copy of
those directories from the upstream checkout. Two exceptions apply, both needed
because this workspace builds documentation with `-D warnings`:

* `src/hpke/check_code.rs`: the `EstablishedHpkeChannel` doc link is qualified
  with `super::`, as the type lives in the parent module.
* `src/olm/session/ratchet.rs`: the `RatchetKey` doc link on the public
  `RatchetPublicKey` is plain code, as `RatchetKey` is private.

Anything outside those two directories (`Cargo.toml`, `.rustfmt.toml`,
`clippy.toml`) is maintained here:

* Dependencies shared with the workspace are inherited from
  `[workspace.dependencies]`; `base64` stays pinned to 0.22 because
  `base64::DecodeError` is part of this crate's public API.
* `rust-version` follows the workspace MSRV rather than upstream's 1.85.
* `.rustfmt.toml` mirrors the upstream formatting configuration so that `src/`
  stays byte-identical.
* The `olm-rs` interoperability tests need the `[patch.crates-io]` entry in the
  workspace manifest, and building them needs `cmake` and a C compiler.
* Only the fuzzing corpora that the unit tests read are vendored; the AFL
  harness crates themselves are not.

## License

[Apache-2.0](./LICENSE)
