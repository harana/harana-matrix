A high-level, batteries-included [Matrix](https://matrix.org/) client library
written in Rust.

This crate seeks to be a general-purpose library for writing software using the
Matrix [Client-Server API](https://spec.matrix.org/latest/client-server-api/)
to communicate with a Matrix homeserver. If you're writing a typical Matrix
client or bot, this is likely the crate you need.

However, the crate is designed in a modular way and depends on several other
lower-level crates. If you're attempting something more custom, you might be
interested in these:

- [`base`]: A no-network-IO client state machine which can be used
  to embed a Matrix client into an existing network stack or to build a new
  Matrix client library on top.
- [`crypto`]: A no-network-IO encryption state machine which can be
  used to add Matrix E2EE support into an existing client or library.

# Getting started

The central component you'll be interacting with is the [`Client`]. A basic use
case will include instantiating the client, logging in as a user, registering
some event handlers and then syncing.

This is demonstrated in the example below.

```rust,no_run
use matrix::{
    Client, config::SyncSettings,
    ruma::{user_id, events::room::message::SyncRoomMessageEvent},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let alice = user_id!("@alice:example.org");
    let client = Client::builder().server_name(alice.server_name()).build().await?;

    // First we need to log in.
    client.matrix_auth().login_username(alice, "password").send().await?;

    client.add_event_handler(|ev: SyncRoomMessageEvent| async move {
        println!("Received a message {:?}", ev);
    });

    // Syncing is important to synchronize the client state with the server.
    // This method will never return unless there is an error.
    client.sync(SyncSettings::default()).await?;

    Ok(())
}
```

More examples can be found in the [examples] directory.

## Crate feature flags

The following crate feature flags are available:

| Feature          | Default | Description                                                                                                                                     |
| ---------------- | :-----: | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `anyhow`         |   No    | Better logging for event handlers that return `anyhow::Result`                                                                                  |
| `e2e-encryption` |   Yes   | End-to-end encryption (E2EE) support                                                                                                            |
| `eyre`           |   No    | Better logging for event handlers that return `eyre::Result`                                                                                    |
| `js`             |   No    | Enables JavaScript API usage on WASM (does nothing on other targets)                                                                            |
| `markdown`       |   No    | Support for sending Markdown-formatted messages                                                                                                 |
| `qrcode`         |   Yes   | QR code verification support                                                                                                                    |
| `sqlite`         |   No    | Persistent storage of state and E2EE data (optionally, if feature `e2e-encryption` is enabled), via SQLite available on system                  |
| `bundled-sqlite` |   No    | Persistent storage of state and E2EE data (optionally, if feature `e2e-encryption` is enabled), via SQLite compiled and bundled with the binary |
| `indexeddb`      |   No    | Persistent storage of state and E2EE data (optionally, if feature `e2e-encryption` is enabled) for browsers, via IndexedDB                      |
| `socks`          |   No    | SOCKS support in the default HTTP client, [`reqwest`]                                                                                           |
| `sso-login`      |   No    | Support for SSO login with a local HTTP server                                                                                                  |
| `reqwest-transport` | Yes  | Send HTTP requests with [`reqwest`], which needs a Tokio runtime                                                                                |
| `tokio-runtime`  |   Yes   | Spawn tasks and sleep on Tokio                                                                                                                  |

[`reqwest`]: https://docs.rs/reqwest/0.11.5/reqwest/index.html

## Running on another async runtime

The SDK does not talk to a concrete async runtime. There are two things it
needs from one, and both can be replaced:

- **Spawning tasks, sleeping and running blocking work.** This is described by
  [`sdk_common::runtime::AsyncRuntime`]. Tokio is used by default (the
  `tokio-runtime` feature); install your own with
  [`sdk_common::runtime::set_runtime`] before creating a [`Client`].
- **Sending HTTP requests.** This is described by [`HttpSend`]. `reqwest` is
  used by default (the `reqwest-transport` feature); hand your own to
  [`ClientBuilder::http_transport`].

Both defaults are on by default, and installing your own runtime with
`set_runtime()` takes precedence over the Tokio one, so most applications need
to do nothing. To build with neither Tokio nor `reqwest`, turn the default
features off:

```toml
matrix = { version = "0.18", default-features = false, features = ["e2e-encryption", "sqlite"] }
```

Note that `tokio` remains a dependency for its synchronisation primitives
(`Mutex`, `RwLock`, `broadcast`, …), which the SDK uses in its public API.
Those contain no runtime, no reactor and no timers, and work on any executor.

## Storage

The SDK does not talk to a concrete database either. It persists its data
through four traits, all of which live in `base` and can be
implemented against any backend:

| Trait             | Holds                                                  |
| ----------------- | ------------------------------------------------------ |
| `StateStore`      | Room state, account data, the send queue                |
| `EventCacheStore` | The persisted event cache (linked chunks)               |
| `MediaStore`      | Cached media, with its retention policy                 |
| `CryptoStore`     | E2EE data, with the `e2e-encryption` feature            |

SQLite (the `sqlite` feature) and IndexedDB (the `indexeddb` feature) are
implementations shipped with the SDK, not requirements. Neither is on by
default: with no store configured the SDK keeps everything in memory, which is
enough for bots and tests, and applications that want persistence pick a
backend explicitly.

```toml
# Persist to SQLite, linking against the system's libsqlite3.
matrix = { version = "0.18", features = ["sqlite"] }
# ...or compile SQLite into the binary instead.
matrix = { version = "0.18", features = ["bundled-sqlite"] }
```

To plug in a backend of your own, there are two entry points on
[`ClientBuilder`]:

- [`ClientBuilder::store_config`] takes a [`StoreConfig`] holding stores you
  have already opened. Use it when opening them is synchronous and infallible.
- [`ClientBuilder::store_provider`] takes a [`StoreProvider`], which the SDK
  calls while building the [`Client`]. Use it when opening the stores is
  asynchronous or fallible; a failure is reported as
  `ClientBuildError::StoreProvider`.

```rust,no_run
use matrix::{
    BoxFuture, Client, StoreProvider, StoreProviderError, config::StoreConfig,
    cross_process_lock::CrossProcessLockConfig, store::MemoryStore,
};

#[derive(Debug)]
struct MyBackend {
    connection_string: String,
}

impl StoreProvider for MyBackend {
    fn open_stores<'a>(
        &'a self,
        cross_process_lock_config: &'a CrossProcessLockConfig,
    ) -> BoxFuture<'a, Result<StoreConfig, StoreProviderError>> {
        Box::pin(async move {
            // Open your own `StateStore`, `EventCacheStore`, `MediaStore` and
            // `CryptoStore` here, however that is done for your backend.
            Ok(StoreConfig::new(cross_process_lock_config.clone())
                .state_store(MemoryStore::new()))
        })
    }
}

# async fn example() -> anyhow::Result<()> {
let client = Client::builder()
    .homeserver_url("http://localhost:8008")
    .store_provider(MyBackend { connection_string: "postgres://...".to_owned() })
    .build()
    .await?;
# anyhow::Ok(())
# }
```

Every store a [`StoreConfig`] is not given falls back to the in-memory
implementation, so a backend can cover only the stores it cares about.

[`Client`]: https://docs.rs/matrix/latest/matrix/struct.Client.html
[`ClientBuilder`]: https://docs.rs/matrix/latest/matrix/struct.ClientBuilder.html
[`ClientBuilder::store_config`]: https://docs.rs/matrix/latest/matrix/struct.ClientBuilder.html#method.store_config
[`ClientBuilder::store_provider`]: https://docs.rs/matrix/latest/matrix/struct.ClientBuilder.html#method.store_provider
[`StoreConfig`]: https://docs.rs/matrix/latest/matrix/config/struct.StoreConfig.html
[`StoreProvider`]: https://docs.rs/matrix/latest/matrix/trait.StoreProvider.html

## Enabling logging

Users of the matrix crate can enable log output by depending on the
`tracing-subscriber` crate and including the following line in their
application (e.g. at the start of `main`):

```rust
tracing_subscriber::fmt::init();
```

The log output is controlled via the `RUST_LOG` environment variable by
setting it to one of the `error`, `warn`, `info`, `debug` or `trace` levels.
The output is printed to stdout.

The `RUST_LOG` variable also supports a more advanced syntax for filtering
log output more precisely, for instance with crate-level granularity. For
more information on this, check out the [tracing_subscriber documentation].

[examples]: https://github.com/matrix-org/matrix-rust-sdk/tree/main/examples/
[tracing_subscriber documentation]: https://tracing.rs/tracing_subscriber/filter/struct.envfilter
[`crypto`]: https://docs.rs/crypto/
[`base`]: https://docs.rs/base/
