# ruma (vendored)

A fork of [ruma](https://github.com/ruma/ruma), inlined into this workspace and
merged into a single crate.

## Provenance

| | |
| --- | --- |
| Upstream | <https://github.com/ruma/ruma> |
| Revision | `bf21677a8fcba04fd01e341809eb5991908441a2` (the `matrix-org/ruma` fork of `0908aaa9847093ecb32bc6edf0af525f726717fd`) |
| License | MIT, see `LICENSE` |

## Layout

Upstream splits its types across several crates. They are modules here:

| upstream crate | module |
| --- | --- |
| `ruma-common` | crate root |
| `ruma-events` | `events` |
| `ruma-client-api` | `api::client` |
| `ruma-federation-api` | `api::federation` |
| `ruma-html` | `html` |
| `ruma-signatures` | `signatures` |

`ruma-macros` and `ruma-identifiers-validation` stay separate crates: the first
is a proc-macro crate, the second is shared between this crate and the macros at
compile time.

## What was dropped

Nothing in this workspace used these, so they are not vendored:

- `ruma-appservice-api`, `ruma-identity-service-api`, `ruma-push-gateway-api`
- `ruma-state-res`
- the `ring-compat` PKCS#8 fallback for keys written by old `ring` versions
- upstream's trybuild UI tests, which asserted macro diagnostics text

## Tests

The vendored test suite needs every feature:

```sh
cargo test -p ruma --features full
```
