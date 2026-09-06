# harana-matrix-common

Everything the client and the server halves of this workspace share: the Matrix
protocol types and algorithms, and the Olm and Megolm ratchets. Both are
vendored forks, inlined here.

## The ruma fork

| | |
| --- | --- |
| Upstream | <https://github.com/ruma/ruma> |
| Revision | `bf21677a8fcba04fd01e341809eb5991908441a2` (the `matrix-org/ruma` fork of `0908aaa9847093ecb32bc6edf0af525f726717fd`) |
| License | MIT, see `LICENSE` |

One module per upstream crate:

| upstream crate | module here |
| --- | --- |
| `ruma-common` | crate root |
| `ruma-events` | `events` |
| `ruma-client-api` | `api::client` |
| `ruma-federation-api` | `api::federation` |
| `ruma-appservice-api` | `api::appservice` |
| `ruma-html` | `html` |
| `ruma-signatures` | `signatures` |
| `ruma-state-res` | `state_res` |
| `ruma-identifiers-validation` | `validation` |

`ruma-macros` is `harana-matrix-macros`: a proc-macro crate cannot be merged
into a normal library. It carries a byte-identical private copy of `validation`,
because it validates identifier literals at expansion time and cannot depend on
this crate; `cargo xtask ci validation-sync` fails if the two drift.

The sources were vendored while all of this was one crate, so they still refer
to each other through `crate::` paths. The crate root carries
`pub use crate as __ruma;` and those paths were rewritten to `crate::__ruma::`,
which lands back at the root and so still resolves.

### What was dropped

Nothing in this workspace used these, so they are not vendored:

- of `ruma-appservice-api`, everything but the registration file format
- `ruma-identity-service-api`, `ruma-push-gateway-api`
- the `ring-compat` PKCS#8 fallback for keys written by old `ring` versions
- upstream's trybuild UI tests, which asserted macro diagnostics text

## The Olm fork

The `olm` module, behind the `olm` feature. See [OLM-README.md](./OLM-README.md)
for provenance and re-sync notes. It is Apache-2.0, which is why this crate is
`MIT AND Apache-2.0`.

## Tests

The vendored ruma test suite needs every feature:

```sh
cargo test -p harana-matrix-common --features full
```
