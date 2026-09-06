# Architecture

## Crates

The workspace publishes four crates. Three are the tiers, and the fourth exists
only because a proc-macro crate cannot be merged into a normal library:

| crate                   | at              | who it is for                                                              |
| ----------------------- | --------------- | -------------------------------------------------------------------------- |
| `harana-matrix-common`  | `crates/common` | Matrix protocol types and algorithms, and the Olm ratchets: used by both sides |
| `harana-matrix-client`  | `crates/client` | the client SDK, its stores, its UI layer and its test helpers                |
| `harana-matrix-server`  | `crates/server` | homeserver-side building blocks                                             |
| `harana-matrix-macros`  | `crates/macros` | the derive and attribute macros the three above are built on                |

Each crate is a merge of what used to be a crate per module, so a module is the
unit of organisation and a feature usually switches one on. A dependency is
`harana-matrix-client` in `Cargo.toml` and `harana_matrix_client::` in Rust.

The SDK is layered like this, top to bottom, all within `harana-matrix-client`
except where noted:

```text
        WASM (external crate matrix-rust-sdk-crypto-wasm)
          /
         /        uniffi
        /        /
       /     bindings (bindings/client-matrix-ffi)
   crypto        |
  bindings       |
      |          |
      |       `ui`
      |           \
      |            \
      |        crate root
      |    /       /
   `crypto`       /
           \     /
      `base`, and the store implementations `sqlite` and `indexeddb`
               |
           `common`
               |
     harana-matrix-common (protocol types, `events`, `api::client`, ...)
```

`MemoryStore` lives in `base` alongside the traits the other two implement.

## `crates/common` — `harana-matrix-common`

Everything the client and the server halves share. It is two vendored forks in
one crate.

### The ruma fork

A trimmed-down fork of [ruma](https://github.com/ruma/ruma), whose crates are
the modules named after them:

| module here            | upstream crate                | contents                                                                            |
| ---------------------- | ----------------------------- | ----------------------------------------------------------------------------------- |
| crate root             | `ruma-common`                 | identifiers, (de)serialization helpers, push rules, canonical JSON, the core request/response traits of `api` |
| `events`               | `ruma-events`                 | the event types                                                                      |
| `api::client`          | `ruma-client-api`             | the client-server endpoints                                                          |
| `api::federation`      | `ruma-federation-api`         | the server-server endpoints                                                          |
| `api::appservice`      | `ruma-appservice-api`         | the appservice registration file format                                              |
| `html`                 | `ruma-html`                   | HTML parsing and sanitizing                                                          |
| `signatures`           | `ruma-signatures`             | digital signatures                                                                   |
| `state_res`            | `ruma-state-res`              | state resolution and PDU authorization                                               |
| `validation`           | `ruma-identifiers-validation` | the identifier grammar the identifiers validate against                              |

The sources were vendored while all of this was a single crate, so they refer to
each other through `crate::...` paths. Rather than rewrite every one of those,
the crate root carries `pub use crate as __ruma;` and those paths were rewritten
to `crate::__ruma::...`, which lands back at the root and so still resolves.

Only the parts this workspace uses are vendored: of the appservice API only the
registration file format, and the identity-service and push-gateway APIs not at
all.

### The Olm fork

The `olm` module is the Olm and Megolm implementation, vendored from
[harana-olm](https://github.com/harana/harana-olm), a fork of
[vodozemac](https://github.com/matrix-org/vodozemac). See
[OLM-README.md](./crates/common/OLM-README.md) for provenance and re-sync notes.
It keeps `base64` 0.22 under the `olm-base64` alias, because its public API
exposes `base64::DecodeError` while the rest of the crate is on 0.23.

The crate is therefore MIT **and** Apache-2.0: the ruma fork is MIT, the
vodozemac fork is Apache-2.0.

### Testing

`testing` switches on the `testing` module, which holds
`init_tracing_for_tests!`. The vendored ruma test suite needs the whole feature
set: `cargo test -p harana-matrix-common --features full`.

## `crates/client` — `harana-matrix-client`

The main crate, and the one most consumers want. Its root is the SDK proper:

- the `Client`, which runs room-independent requests: logging in/out, creating
  rooms, running sync, and so on.
- the `Room`, which represents a room and its state (notably via the observable
  `RoomInfo`), and runs the room-specific queries, notably sending events.

Everything under it is a module, each behind a feature except the three that are
always compiled in:

| module             | feature                      | contents                                                                     |
| ------------------ | ---------------------------- | ---------------------------------------------------------------------------- |
| `common`           | always                       | helpers used by most of the other modules; nearly a leaf                      |
| `base`             | always                       | the _sans I/O_ base data types, the `StateStore` / `EventCacheStore` traits, and `MemoryStore` |
| `store_encryption` | always                       | low-level primitives for encrypting, decrypting and hashing stored values     |
| `crypto`           | `e2e-encryption`             | the _sans I/O_ end-to-end encryption state machine and its `CryptoStore` trait |
| `qrcode`           | `qrcode`                     | QR codes for interactive verification, used by `crypto`                       |
| `sqlite`           | `sqlite`                     | `EventCacheStore`, `StateStore` and `CryptoStore` over SQLite                 |
| `indexeddb`        | `indexeddb`                  | the same three over IndexedDB, for browsers via WebAssembly                   |
| `search`           | `experimental-search-core`   | the client-side full-text index over a room's timeline, Tantivy behind `experimental-search` |
| `ui`               | `ui`                         | the high-level services: `RoomListService`, `EncryptionSyncService`, `SyncService`, `Timeline` |
| `contentscanner`   | `contentscanner`             | a client for a [content scanner], so media is checked for malware first       |
| `ruma_client`      | `ruma-client`                | a minimal Matrix client over the protocol types, vendored from [ruma-client]; independent of the rest |
| `test`             | `testing`                    | the test helpers the whole workspace uses                                     |

[content scanner]: https://github.com/element-hq/matrix-content-scanner-python
[ruma-client]: https://github.com/ruma/ruma

`_crypto` is the module gate `e2e-encryption` and the store sub-features hang
off; do not enable it directly.

## `crates/server` — `harana-matrix-server`

Homeserver-side building blocks, mostly ported from
[tuwunel](https://github.com/matrix-construct/tuwunel). None of them depend on
the client SDK. Each module is behind a feature of the same name, all on by
default, and documented by `crates/server/docs/<module>.md`.

| module        | contents                                                                        |
| ------------- | ------------------------------------------------------------------------------- |
| `appservice`  | application service registration and namespace matching                          |
| `resolver`    | server name resolution: the `.well-known` and SRV ladder of the server-server spec |
| `state_res`   | asynchronous, store-backed adapters over `harana_matrix_common::state_res`       |
| `store_codec` | a compact, order-preserving binary codec for key-value store records             |
| `thumbnail`   | thumbnail generation for Matrix media, with a bounded decode budget              |

## `crates/macros` — `harana-matrix-macros`

The derive and attribute macros, upstream's `ruma-macros` plus `#[async_test]`
and the `#[uniffi_export]` helper the FFI bindings use. They are pooled into one
crate because a proc-macro crate can only export proc macros, so each of them
would otherwise need a crate of its own.

It also carries a private copy of `harana-matrix-common`'s `validation` module:
the macros validate identifier literals at expansion time, so they need the same
logic, and a proc-macro crate cannot depend on the crate that depends on it. The
two copies are kept byte-identical, and `cargo xtask ci validation-sync` fails if
they drift. The compat features that change validation behaviour are mirrored
here and forwarded by `harana-matrix-common`.

## `bindings/client-crypto-ffi/`

FFI bindings for the crypto module, used in a Web browser context via
WebAssembly. These use `wasm-bindgen` to generate the bindings. These bindings
are used in Element Web and the legacy Element apps, as of 2024-11-07.

## `bindings/client-matrix-ffi/`

FFI bindings for important concepts in the `ui` module and the SDK proper,
generated with [UniFFI](https://github.com/mozilla/uniffi-rs) and to be used
from other languages like Swift/Go/Kotlin. These bindings are used in the
ElementX apps, as of 2024-11-07.

## `testing/client-integration-testing/`

Fully-fledged integration tests that require spawning a Synapse instance to run.
A docker-compose setup is provided to ease running the tests, and it is
compatible for running with Podman too.

## Inspiration

This document has been inspired by the reading of this
[blog post](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html).
