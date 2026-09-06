# Architecture

The SDK is split into multiple layers:

```text
    WASM (external crate matrix-rust-sdk-crypto-wasm)
      /
     /     uniffi
    /     /
   /     bindings (matrix-ffi)
 crypto   |
bindings  |
   |      |
   |     UI (ui)
   |      \
   |       \
   |   main (matrix)
   | /     /
crypto    /
     \   /
      store (base, + all the store impls)
        |
      common (sdk-common)
        |
   protocol types (ruma, over common/events/client-api/...)
```

Where the store implementations are `sqlite` and
`indexeddb` as well as `MemoryStore` which is defined in
`base`.

## `crates/matrix`

This is the main crate, and one that is expected to be used by most consumers.
Notable data types include:

- the `Client`, which can run room-independent requests: logging in/out,
  creating rooms, running sync, etc.
- the `Room`, which represents a room and its state (notably via the observable
  `RoomInfo`), and allows running queries that are room-specific, notably
  sending events.

## `crates/ruma` and the protocol type crates

`ruma` is a facade over a vendored, trimmed-down fork of
[ruma](https://github.com/ruma/ruma). It owns no types of its own: it re-exports
one crate per upstream crate, under the module name upstream uses, so consumers
keep writing `ruma::events::...`, `ruma::api::client::...` and so on.

| crate here                          | upstream crate                | contents                                                                            |
| ----------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------- |
| `crates/common`                     | `ruma-common`                 | identifiers, (de)serialization helpers, push rules, canonical JSON, the core request/response traits of `api` |
| `crates/events`                     | `ruma-events`                 | the event types, at `ruma::events`                                                   |
| `crates/client-api`                 | `ruma-client-api`             | the client-server endpoints, at `ruma::api::client`                                  |
| `crates/federation-api`             | `ruma-federation-api`         | the server-server endpoints, at `ruma::api::federation`                              |
| `crates/appservice-api`             | `ruma-appservice-api`         | the appservice registration file format, at `ruma::api::appservice`                  |
| `crates/html`                       | `ruma-html`                   | HTML parsing and sanitizing, at `ruma::html`                                          |
| `crates/signatures`                 | `ruma-signatures`             | digital signatures, at `ruma::signatures`                                            |
| `crates/state-res`                  | `ruma-state-res`              | state resolution and PDU authorization, at `ruma::state_res`                          |
| `crates/ruma-macros`                | `ruma-macros`                 | the derive and attribute macros the crates above are built on                        |
| `crates/ruma-identifiers-validation` | `ruma-identifiers-validation` | the identifier grammar the `common` identifiers validate against                     |

The sources were vendored while all of this was a single crate, so they refer to
each other through `crate::...` paths. Rather than rewrite every one of those, each
split crate carries a `__ruma` shim module that reproduces the old crate root,
and the paths were rewritten to `crate::__ruma::...`. `common` needs no shim: it is
the old crate root.

Only the parts this workspace uses are vendored: of the appservice API only the
registration file format, and the identity-service and push-gateway APIs not at
all.

## `crates/server-axum`

The route table of a Matrix homeserver, as an [axum](https://docs.rs/axum)
router. It is built on the endpoint metadata of the protocol type crates above:
every client-server and server-server endpoint they define is registered, on
every path it has ever had, with the extractor and response types that turn HTTP
requests into Ruma request types and Ruma response types back into HTTP
responses. It contains no server logic of its own; endpoints without a handler
answer `404 M_UNRECOGNIZED`.

## `crates/harana-olm`

The Olm and Megolm implementation, vendored from
[harana-olm](https://github.com/harana/harana-olm) and published under the
`vodozemac` package name, so consumers keep using `vodozemac::` paths. See
[its README](./crates/harana-olm/README.md) for provenance and re-sync notes.

## `crates/base`

A _sans I/O_ crate to represent the base data types persisted in the SDK. No
network or storage I/O happens in this crate, although it defines traits
(`StateStore` and `EventCacheStore`) representing storage backends, as well as
dummy in-memory implementations of these traits.

## `crates/sdk-common`

Common helpers used by most of the other crates; almost a leaf in the dependency
tree of our own crates (the only crate it's using is test helpers).

## `crates/crypto`

A _sans I/O_ implementation of a state machine that handles end-to-end
encryption for Matrix clients. It defines a `CryptoStore` trait representing
storage backends that will perform the actual storage I/O later, as well as a
dummy in-memory implementation of this trait.

## `crates/indexeddb`

Implementations of `EventCacheStore`, `StateStore` and `CryptoStore` for a
indexeddb backend (for use in Web browsers, via WebAssembly).

## `crates/qrcode`

Implementation of QR codes for interactive verifications, used in the crypto
crate.

## `crates/sqlite`

Implementations of `EventCacheStore`, `StateStore` and `CryptoStore` for a
SQLite backend.

## `crates/store-encryption`

Low-level primitives for encrypting/decrypting/hashing values. Store
implementations that implement encryption at rest can use those primitives.

## `crates/ui`

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

## `bindings/crypto-ffi/`

FFI bindings for the crypto crate, used in a Web browser context via
WebAssembly. These use `wasm-bindgen` to generate the bindings. These bindings
are used in Element Web and the legacy Element apps, as of 2024-11-07.

## `bindings/matrix-ffi/`

FFI bindings for important concepts in `ui` and `matrix`,
generated with [UniFFI](https://github.com/mozilla/uniffi-rs) and to be used
from other languages like Swift/Go/Kotlin. These bindings are used in the
ElementX apps, as of 2024-11-07.

## `bindings/matrix-ffi-macros/`

Macros used in `bindings/matrix-ffi`.

## `testing/sdk-test/`

Common test helpers, used by all the other crates.

## `testing/sdk-test-macros/`

Implementation of the `#[async_test]` test macro.

## `testing/integration-testing/`

Fully-fledged integration tests that require spawning a Synapse instance to run.
A docker-compose setup is provided to ease running the tests, and it is
compatible for running with Podman too.

## Inspiration

This document has been inspired by the reading of this
[blog post](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html).
