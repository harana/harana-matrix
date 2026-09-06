# Architecture

## Crate naming

Every crate in the workspace carries a tier prefix that says who it is for:

| prefix    | tier                                                                       |
| --------- | -------------------------------------------------------------------------- |
| `common-` | Matrix protocol types and algorithms, and shared test helpers: used by both sides |
| `client-` | the client SDK, its stores, its UI layer and its FFI bindings                |
| `server-` | homeserver-side building blocks                                             |

The prefix is part of the package name, so a dependency is `client-crypto` in
`Cargo.toml` and `client_crypto::` in Rust. Modules re-exported from a crate keep
their own names: the facade still exposes `common_ruma::events`, and
`client_common::ruma` is still spelled `ruma`.

The SDK is split into multiple layers:

```text
        WASM (external crate matrix-rust-sdk-crypto-wasm)
          /
         /        uniffi
        /        /
       /     bindings (client-matrix-ffi)
   crypto        |
  bindings       |
      |          |
      |    UI (client-ui)
      |           \
      |            \
      |      main (client-matrix)
      |    /       /
  client-crypto   /
           \     /
     store (client-base, + all the store impls)
               |
     common helpers (client-common)
               |
     protocol types (common-ruma, over common-types,
     common-events, common-client-api, ...)
```

Where the store implementations are `client-sqlite` and
`client-indexeddb` as well as `MemoryStore` which is defined in
`client-base`.

## `crates/client-matrix`

This is the main crate, and one that is expected to be used by most consumers.
Notable data types include:

- the `Client`, which can run room-independent requests: logging in/out,
  creating rooms, running sync, etc.
- the `Room`, which represents a room and its state (notably via the observable
  `RoomInfo`), and allows running queries that are room-specific, notably
  sending events.

## `crates/common-ruma` and the protocol type crates

`common-ruma` is a facade over a vendored, trimmed-down fork of
[ruma](https://github.com/ruma/ruma). It owns no types of its own: it re-exports
one crate per upstream crate, under the module name upstream uses, so consumers
keep writing `common_ruma::events::...`, `common_ruma::api::client::...` and so
on.

| crate here                          | upstream crate                | contents                                                                            |
| ----------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------- |
| `crates/common-types`               | `ruma-common`                 | identifiers, (de)serialization helpers, push rules, canonical JSON, the core request/response traits of `api` |
| `crates/common-events`              | `ruma-events`                 | the event types, at `common_ruma::events`                                            |
| `crates/common-client-api`          | `ruma-client-api`             | the client-server endpoints, at `common_ruma::api::client`                           |
| `crates/common-federation-api`      | `ruma-federation-api`         | the server-server endpoints, at `common_ruma::api::federation`                       |
| `crates/common-appservice-api`      | `ruma-appservice-api`         | the appservice registration file format, at `common_ruma::api::appservice`           |
| `crates/common-html`                | `ruma-html`                   | HTML parsing and sanitizing, at `common_ruma::html`                                  |
| `crates/common-signatures`          | `ruma-signatures`             | digital signatures, at `common_ruma::signatures`                                     |
| `crates/common-state-res`           | `ruma-state-res`              | state resolution and PDU authorization, at `common_ruma::state_res`                  |
| `crates/common-macros`              | `ruma-macros`                 | the derive and attribute macros the crates above are built on                        |
| `crates/common-identifiers-validation` | `ruma-identifiers-validation` | the identifier grammar the `common-types` identifiers validate against            |

The sources were vendored while all of this was a single crate, so they refer to
each other through `crate::...` paths. Rather than rewrite every one of those, each
split crate carries a `__ruma` shim module that reproduces the old crate root,
and the paths were rewritten to `crate::__ruma::...`. `common-types` needs no
shim: it is the old crate root.

Only the parts this workspace uses are vendored: of the appservice API only the
registration file format, and the identity-service and push-gateway APIs not at
all.

## `crates/common-olm`

The Olm and Megolm implementation, vendored from
[harana-olm](https://github.com/harana/harana-olm), a fork of
[vodozemac](https://github.com/matrix-org/vodozemac). Consumers reach it as
`common_olm::`. See [its README](./crates/common-olm/README.md) for provenance
and re-sync notes.

## `crates/client-base`

A _sans I/O_ crate to represent the base data types persisted in the SDK. No
network or storage I/O happens in this crate, although it defines traits
(`StateStore` and `EventCacheStore`) representing storage backends, as well as
dummy in-memory implementations of these traits.

## `crates/client-common`

Common helpers used by most of the other client crates; almost a leaf in the dependency
tree of our own crates (the only crate it's using is test helpers).

## `crates/client-crypto`

A _sans I/O_ implementation of a state machine that handles end-to-end
encryption for Matrix clients. It defines a `CryptoStore` trait representing
storage backends that will perform the actual storage I/O later, as well as a
dummy in-memory implementation of this trait.

## `crates/client-indexeddb`

Implementations of `EventCacheStore`, `StateStore` and `CryptoStore` for a
indexeddb backend (for use in Web browsers, via WebAssembly).

## `crates/client-qrcode`

Implementation of QR codes for interactive verifications, used in the
`client-crypto` crate.

## `crates/client-sqlite`

Implementations of `EventCacheStore`, `StateStore` and `CryptoStore` for a
SQLite backend.

## `crates/client-store-encryption`

Low-level primitives for encrypting/decrypting/hashing values. Store
implementations that implement encryption at rest can use those primitives.

## `crates/client-ui`

Very high-level primitives implementing the best practices and cutting-edge
Matrix tech:

- `EncryptionSyncService`: a specialized service running simplified sliding sync
  (MSC4186) for everything related to crypto and E2EE for the current `Client`.
- `RoomListService`: a specialized service running simplified sliding sync
  (MSC4186) for retrieving the list of current rooms, and exposing its entries.
- `SyncService`: a wrapper for the two previous services, coordinating their
  running and shutting down.
- `Timeline`: a high-level view for a `Room`'s timeline of events, grouping
  related events (aggregations) into single timeline items.

## `crates/client-contentscanner`

An optional client for a [content scanner], so media can be checked for malware
before it reaches the user.

[content scanner]: https://github.com/element-hq/matrix-content-scanner-python

## `crates/client-search`

The client-side full-text search index over a room's timeline, with a Tantivy
backend behind the `tantivy` feature and a `backend` module of traits for
supplying an engine of your own.

## `crates/client-ruma`

A minimal Matrix client library over the protocol types, vendored from
[ruma-client](https://github.com/ruma/ruma). It is independent of the rest of
the client SDK.

## The `server-` crates

Homeserver-side building blocks, mostly ported from
[tuwunel](https://github.com/matrix-construct/tuwunel). None of them depend on
the client SDK.

| crate                       | contents                                                                        |
| --------------------------- | ------------------------------------------------------------------------------- |
| `crates/server-appservice`  | application service registration and namespace matching                          |
| `crates/server-resolver`    | server name resolution: the `.well-known` and SRV ladder of the server-server spec |
| `crates/server-state-res`   | asynchronous, store-backed adapters over `common_ruma::state_res`                |
| `crates/server-store-codec` | a compact, order-preserving binary codec for key-value store records             |
| `crates/server-thumbnail`   | thumbnail generation for Matrix media, with a bounded decode budget              |

## `bindings/client-crypto-ffi/`

FFI bindings for the crypto crate, used in a Web browser context via
WebAssembly. These use `wasm-bindgen` to generate the bindings. These bindings
are used in Element Web and the legacy Element apps, as of 2024-11-07.

## `bindings/client-matrix-ffi/`

FFI bindings for important concepts in `client-ui` and `client-matrix`,
generated with [UniFFI](https://github.com/mozilla/uniffi-rs) and to be used
from other languages like Swift/Go/Kotlin. These bindings are used in the
ElementX apps, as of 2024-11-07.

## `bindings/client-matrix-ffi-macros/`

Macros used in `bindings/client-matrix-ffi`.

## `testing/common-test/`

Common test helpers, used by all the other crates.

## `testing/common-test-macros/`

Implementation of the `#[async_test]` test macro.

## `testing/common-test-utils/`

Smaller test utilities shared by the crates' own test suites.

## `testing/client-integration-testing/`

Fully-fledged integration tests that require spawning a Synapse instance to run.
A docker-compose setup is provided to ease running the tests, and it is
compatible for running with Podman too.

## Inspiration

This document has been inspired by the reading of this
[blog post](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html).
