# harana-matrix-macros

Procedural macros for the [`harana-matrix-common`][common] crate.

This crate is an implementation detail of `harana-matrix-common` and is not
meant to be depended on directly. Every macro it exports is re-exported from
`harana-matrix-common`, which is where they are documented.

It carries a private, byte-identical copy of `harana-matrix-common`'s
`validation` module, so that an identifier literal checked at macro expansion
time is validated exactly like one parsed at runtime. `cargo xtask ci
validation-sync` asserts the two copies have not drifted; the compat features
that change validation behaviour are mirrored here and forwarded by
`harana-matrix-common`.

[common]: https://crates.io/crates/harana-matrix-common
