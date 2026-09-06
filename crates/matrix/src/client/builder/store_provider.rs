// Copyright 2026 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The storage backend the SDK persists its data in.

use std::{fmt, sync::Arc};

use base::store::StoreConfig;
use sdk_common::{
    BoxFuture, SendOutsideWasm, SyncOutsideWasm, cross_process_lock::CrossProcessLockConfig,
};

/// The error a [`StoreProvider`] reports when it cannot open its stores.
pub type StoreProviderError = Box<dyn std::error::Error + Send + Sync>;

/// Something that can open the stores the SDK persists its data in.
///
/// The SDK doesn't talk to a concrete database. It only needs the four store
/// traits a [`StoreConfig`] is made of, all of which live in `base`
/// and can be implemented against any backend:
///
/// * [`StateStore`] for room state, account data and the send queue,
/// * [`EventCacheStore`] for the persisted event cache,
/// * [`MediaStore`] for cached media,
/// * `CryptoStore` for E2EE data (with the `e2e-encryption` feature).
///
/// SQLite (the `sqlite` feature) and IndexedDB (the `indexeddb` feature) are
/// implementations shipped with the SDK, not requirements: with neither feature
/// enabled the SDK falls back to in-memory stores, and an application can plug
/// in a backend of its own instead.
///
/// There are two ways to do that:
///
/// * [`ClientBuilder::store_config`] takes a [`StoreConfig`] holding stores the
///   caller has already opened. Use it when opening the stores is synchronous,
///   infallible, or already done by the time the [`Client`] is built.
///
/// * [`ClientBuilder::store_provider`] takes an implementation of this trait,
///   and calls it while building the [`Client`]. Use it when opening the stores
///   is asynchronous or fallible: the provider is handed the client's
///   [`CrossProcessLockConfig`], and the error it returns is reported as
///   [`ClientBuildError::StoreProvider`].
///
/// # Example
///
/// ```no_run
/// use matrix::{
///     BoxFuture, Client, StoreProvider, StoreProviderError,
///     config::StoreConfig,
///     cross_process_lock::CrossProcessLockConfig,
///     store::MemoryStore,
/// };
/// # async fn open_my_state_store(_: &str) -> Result<MemoryStore, StoreProviderError> {
/// #     Ok(MemoryStore::new())
/// # }
///
/// #[derive(Debug)]
/// struct MyBackend {
///     connection_string: String,
/// }
///
/// impl StoreProvider for MyBackend {
///     fn open_stores<'a>(
///         &'a self,
///         cross_process_lock_config: &'a CrossProcessLockConfig,
///     ) -> BoxFuture<'a, Result<StoreConfig, StoreProviderError>> {
///         Box::pin(async move {
///             let state_store = open_my_state_store(&self.connection_string).await?;
///
///             Ok(StoreConfig::new(cross_process_lock_config.clone())
///                 .state_store(state_store))
///         })
///     }
/// }
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = Client::builder()
///     .homeserver_url("http://localhost:8008")
///     .store_provider(MyBackend { connection_string: "postgres://...".to_owned() })
///     .build()
///     .await?;
/// # anyhow::Ok(())
/// # }
/// ```
///
/// [`Client`]: crate::Client
/// [`ClientBuildError::StoreProvider`]: crate::ClientBuildError::StoreProvider
/// [`ClientBuilder::store_config`]: crate::ClientBuilder::store_config
/// [`ClientBuilder::store_provider`]: crate::ClientBuilder::store_provider
/// [`EventCacheStore`]: base::event_cache::store::EventCacheStore
/// [`MediaStore`]: base::media::store::MediaStore
/// [`StateStore`]: base::store::StateStore
pub trait StoreProvider: fmt::Debug + SendOutsideWasm + SyncOutsideWasm + 'static {
    /// Open the stores and describe them as a [`StoreConfig`].
    ///
    /// This is called once, while a [`Client`](crate::Client) is being built.
    ///
    /// # Arguments
    ///
    /// * `cross_process_lock_config` - The cross-process lock configuration the
    ///   client was built with, to pass to [`StoreConfig::new`]. Backends that
    ///   hold their own locks can ignore what it holds, but must still pass it
    ///   along, as the returned [`StoreConfig`] applies it to the event cache
    ///   and media stores.
    fn open_stores<'a>(
        &'a self,
        cross_process_lock_config: &'a CrossProcessLockConfig,
    ) -> BoxFuture<'a, Result<StoreConfig, StoreProviderError>>;
}

impl StoreProvider for Arc<dyn StoreProvider> {
    fn open_stores<'a>(
        &'a self,
        cross_process_lock_config: &'a CrossProcessLockConfig,
    ) -> BoxFuture<'a, Result<StoreConfig, StoreProviderError>> {
        (**self).open_stores(cross_process_lock_config)
    }
}
