# common-ruma (vendored)

A fork of [ruma](https://github.com/ruma/ruma), inlined into this workspace.
This crate is the facade: it holds no types of its own, and re-exports the
crates below under the module names upstream uses.

## Provenance

| | |
| --- | --- |
| Upstream | <https://github.com/ruma/ruma> |
| Revision | `bf21677a8fcba04fd01e341809eb5991908441a2` (the `matrix-org/ruma` fork of `0908aaa9847093ecb32bc6edf0af525f726717fd`) |
| License | MIT, see `LICENSE` |

## Layout

One crate per upstream crate, re-exported here:

| upstream crate | crate here | path here |
| --- | --- | --- |
| `ruma-common` | `common-types` | crate root |
| `ruma-events` | `common-events` | `common_ruma::events` |
| `ruma-client-api` | `common-client-api` | `common_ruma::api::client` |
| `ruma-federation-api` | `common-federation-api` | `common_ruma::api::federation` |
| `ruma-appservice-api` | `common-appservice-api` | `common_ruma::api::appservice` |
| `ruma-html` | `common-html` | `common_ruma::html` |
| `ruma-signatures` | `common-signatures` | `common_ruma::signatures` |
| `ruma-state-res` | `common-state-res` | `common_ruma::state_res` |

`ruma-macros` is `common-macros` and `ruma-identifiers-validation` is
`common-identifiers-validation`: the first is a proc-macro crate, the second is
shared between `common-types` and the macros at compile time.

The sources were vendored while all of this was one crate, so they still refer
to each other through `crate::` paths. Each split crate carries a `__ruma` shim
module that reproduces the old crate root, and those paths were rewritten to
`crate::__ruma::`. `common-types` needs no shim: it is the old crate root.

## What was dropped

Nothing in this workspace used these, so they are not vendored:

- of `ruma-appservice-api`, everything but the registration file format
- `ruma-identity-service-api`, `ruma-push-gateway-api`
- the `ring-compat` PKCS#8 fallback for keys written by old `ring` versions
- upstream's trybuild UI tests, which asserted macro diagnostics text

## Tests

The vendored test suite needs every feature:

```sh
cargo test -p ruma --features full
```

Each split crate also carries the unit tests that came with its sources:

```sh
cargo test -p common -p events -p client-api --all-features
```
