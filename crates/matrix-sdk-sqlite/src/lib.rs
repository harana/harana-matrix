// Copyright 2022 The Matrix.org Foundation C.I.C.
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

#![cfg_attr(
    not(any(
        feature = "state-store",
        feature = "crypto-store",
        feature = "event-cache-store",
        feature = "media-store"
    )),
    allow(dead_code, unused_imports)
)]

mod connection;
#[cfg(feature = "crypto-store")]
mod crypto_store;
mod encryption;
mod error;
#[cfg(feature = "event-cache-store")]
mod event_cache_store;
mod fs;
#[cfg(feature = "media-store")]
mod media_store;
mod recovery;
#[cfg(feature = "state-store")]
mod state_store;
mod sync_wrapper;
mod utils;
use std::{
    cmp::max,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use deadpool::managed::PoolConfig;
use matrix_sdk_store_encryption::{StoreCipherProvider, StoreCodec};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use self::encryption::EncryptionConfig;
pub use self::encryption::SecretStoreCipherProvider;

/// The cipher and codec traits [`SqliteStoreConfig`] takes, re-exported so
/// that using them does not mean depending on
/// [`matrix_sdk_store_encryption`] directly.
pub mod pluggable {
    pub use matrix_sdk_store_encryption::{
        CodecError, JsonCodec, MessagePackCodec, StoreCipherBackend, StoreCipherProvider,
        StoreCodec, StoreCodecExt,
    };
}

#[cfg(feature = "crypto-store")]
pub use self::crypto_store::SqliteCryptoStore;
pub use self::error::OpenStoreError;
#[cfg(feature = "event-cache-store")]
pub use self::event_cache_store::SqliteEventCacheStore;
#[cfg(feature = "media-store")]
pub use self::media_store::SqliteMediaStore;
#[cfg(feature = "state-store")]
pub use self::state_store::{DATABASE_NAME as STATE_STORE_DATABASE_NAME, SqliteStateStore};

#[cfg(test)]
matrix_sdk_test_utils::init_tracing_for_tests!();

/// The `tracing` targets this store's modules log under.
///
/// Clients that let their users tune log levels per SDK component read these
/// rather than spelling out module paths, so that swapping the store backend
/// swaps its log targets with it.
pub mod log_targets {
    /// The target [`SqliteEventCacheStore`] logs under.
    ///
    /// [`SqliteEventCacheStore`]: crate::SqliteEventCacheStore
    pub const EVENT_CACHE_STORE: &str = "matrix_sdk_sqlite::event_cache_store";

    /// The target [`SqliteStateStore`] logs under.
    ///
    /// [`SqliteStateStore`]: crate::SqliteStateStore
    pub const STATE_STORE: &str = "matrix_sdk_sqlite::state_store";

    /// The target [`SqliteCryptoStore`] logs under.
    ///
    /// [`SqliteCryptoStore`]: crate::SqliteCryptoStore
    pub const CRYPTO_STORE: &str = "matrix_sdk_sqlite::crypto_store";

    /// The target [`SqliteMediaStore`] logs under.
    ///
    /// [`SqliteMediaStore`]: crate::SqliteMediaStore
    pub const MEDIA_STORE: &str = "matrix_sdk_sqlite::media_store";
}

/// An enum used to store the secret that gives access to a store
#[derive(Clone, Debug, PartialEq, Zeroize, ZeroizeOnDrop)]
pub enum Secret {
    // Cryptographic key used to open the store
    Key(Box<[u8; 32]>),
    // Passphrase used to open the store
    PassPhrase(Zeroizing<String>),
}

/// A configuration structure used for opening a store.
#[derive(Clone)]
pub struct SqliteStoreConfig {
    /// Path to the database, without the file name.
    path: PathBuf,
    /// Secret to open the store, if any
    secret: Option<Secret>,
    /// How the store encrypts and serializes what it writes.
    encryption: EncryptionConfig,
    /// The pool configuration for [`deadpool`].
    pool_config: PoolConfig,
    /// The runtime configuration to apply when opening an SQLite connection.
    runtime_config: RuntimeConfig,
}

impl fmt::Debug for SqliteStoreConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SqliteStoreConfig")
            .field("path", &self.path)
            .field("encryption", &self.encryption)
            .field("pool_config", &self.pool_config)
            .field("runtime_config", &self.runtime_config)
            .finish_non_exhaustive()
    }
}

/// The minimum size of the connections pool.
///
/// We need at least 2 connections: one connection for write operations, and one
/// connection for read operations.
const POOL_MINIMUM_SIZE: usize = 2;

impl SqliteStoreConfig {
    /// Create a new [`SqliteStoreConfig`] with a path representing the
    /// directory containing the store database.
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            path: path.as_ref().to_path_buf(),
            pool_config: PoolConfig::new(max(POOL_MINIMUM_SIZE, num_cpus::get_physical() * 4)),
            runtime_config: RuntimeConfig::default(),
            secret: None,
            encryption: EncryptionConfig::default(),
        }
    }

    /// Similar to [`SqliteStoreConfig::new`], but with defaults tailored for a
    /// low memory usage environment.
    ///
    /// The following defaults are set:
    ///
    /// * The `pool_max_size` is set to the number of physical CPU, so one
    ///   connection per physical thread,
    /// * The `cache_size` is set to 500Kib,
    /// * The `journal_size_limit` is set to 2Mib.
    pub fn with_low_memory_config<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::new(path)
            // Maximum one connection per physical thread.
            .pool_max_size(num_cpus::get_physical())
            // Cache size is 500Kib.
            .cache_size(500_000)
            // Journal size limit is 2Mib.
            .journal_size_limit(2_000_000)
    }

    /// Override the path.
    pub fn path<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.path = path.as_ref().to_path_buf();
        self
    }

    /// Define the passphrase if the store is encoded.
    pub fn passphrase(mut self, passphrase: Option<&str>) -> Self {
        self.secret =
            passphrase.map(|passphrase| Secret::PassPhrase(Zeroizing::new(passphrase.to_owned())));
        self
    }

    /// Define the key if the store is encoded.
    pub fn key(mut self, key: Option<&[u8; 32]>) -> Self {
        self.secret = key.map(|key| Secret::Key(Box::new(*key)));
        self
    }

    /// Encrypt the store with a cipher of your own, instead of the default one
    /// derived from [`Self::passphrase`] or [`Self::key`].
    ///
    /// Use this to keep the store's key material somewhere the process cannot
    /// leak it — an OS keychain, a Secure Enclave, an HSM, a KMS — or to
    /// encrypt with a cipher suite of your choosing.
    ///
    /// This takes precedence over any secret set on this config: when a
    /// provider is installed, [`Self::passphrase`] and [`Self::key`] are
    /// ignored. Passing `None` restores the default behaviour.
    ///
    /// Swapping the cipher of a store that already holds data makes that data
    /// unreadable, so this has to be decided before the store is first
    /// opened.
    pub fn cipher_provider(mut self, provider: Option<Arc<dyn StoreCipherProvider>>) -> Self {
        self.encryption.cipher_provider = provider;
        self
    }

    /// Write opaque values in a format of your own, instead of MessagePack.
    ///
    /// This codec is used for the store's opaque value columns, and for the
    /// envelope encrypted values are wrapped in.
    ///
    /// Swapping the codec of a store that already holds data makes that data
    /// unreadable, so this has to be decided before the store is first
    /// opened.
    pub fn value_codec(mut self, codec: Arc<dyn StoreCodec>) -> Self {
        self.encryption.value_codec = codec;
        self
    }

    /// Write Matrix payloads in a format of your own, instead of JSON.
    ///
    /// Note that other clients sharing this database, and other parts of the
    /// SDK, expect to find JSON in these columns.
    ///
    /// Swapping the codec of a store that already holds data makes that data
    /// unreadable, so this has to be decided before the store is first
    /// opened.
    pub fn json_codec(mut self, codec: Arc<dyn StoreCodec>) -> Self {
        self.encryption.json_codec = codec;
        self
    }

    /// Define the maximum pool size for [`deadpool`].
    ///
    /// See [`deadpool::managed::PoolConfig::max_size`] to learn more.
    pub fn pool_max_size(mut self, max_size: usize) -> Self {
        self.pool_config.max_size = max(POOL_MINIMUM_SIZE, max_size);
        self
    }

    /// Optimize the database.
    ///
    /// The SQLite documentation recommends to run this regularly and after any
    /// schema change. The easiest is to do it consistently when the store is
    /// constructed, after eventual migrations.
    ///
    /// See [`PRAGMA optimize`] to learn more.
    ///
    /// The default value is `true`.
    ///
    /// [`PRAGMA optimize`]: https://www.sqlite.org/pragma.html#pragma_optimize
    pub fn optimize(mut self, optimize: bool) -> Self {
        self.runtime_config.optimize = optimize;
        self
    }

    /// Define the maximum size in **bytes** the SQLite cache can use.
    ///
    /// See [`PRAGMA cache_size`] to learn more.
    ///
    /// The default value is 2Mib.
    ///
    /// [`PRAGMA cache_size`]: https://www.sqlite.org/pragma.html#pragma_cache_size
    pub fn cache_size(mut self, cache_size: u32) -> Self {
        self.runtime_config.cache_size = cache_size;
        self
    }

    /// Limit the size of the WAL file, in **bytes**.
    ///
    /// By default, while the DB connections of the databases are open, [the
    /// size of the WAL file can keep increasing][size_wal_file] depending on
    /// the size needed for the transactions. A critical case is `VACUUM`
    /// which basically writes the content of the DB file to the WAL file
    /// before writing it back to the DB file, so we end up taking twice the
    /// size of the database.
    ///
    /// By setting this limit, the WAL file is truncated after its content is
    /// written to the database, if it is bigger than the limit.
    ///
    /// See [`PRAGMA journal_size_limit`] to learn more. The value `limit`
    /// corresponds to `N` in `PRAGMA journal_size_limit = N`.
    ///
    /// The default value is 10Mib.
    ///
    /// [size_wal_file]: https://www.sqlite.org/wal.html#avoiding_excessively_large_wal_files
    /// [`PRAGMA journal_size_limit`]: https://www.sqlite.org/pragma.html#pragma_journal_size_limit
    pub fn journal_size_limit(mut self, limit: u32) -> Self {
        self.runtime_config.journal_size_limit = limit;
        self
    }

    /// Define the `synchronous` behaviour of the database, i.e. how
    /// aggressively SQLite flushes data to disk.
    ///
    /// The database is always opened in [WAL mode][wal]. With
    /// [`Synchronous::Full`], every commit triggers an `fsync`, which can
    /// be a significant performance bottleneck, in particular on spinning
    /// disks or RAID arrays. [`Synchronous::Normal`] only syncs at
    /// checkpoints, which is a lot cheaper while still being safe against
    /// application crashes; data can only be lost after an OS crash or a
    /// power loss.
    ///
    /// See [`PRAGMA synchronous`] to learn more.
    ///
    /// When this is never called, each store picks its own default:
    /// [`Synchronous::Normal`] for the state, event cache and media stores,
    /// since they only hold data that can be resynchronized from the
    /// homeserver, and [`Synchronous::Full`] for the crypto store, since
    /// losing encryption keys is not recoverable.
    ///
    /// [wal]: https://www.sqlite.org/wal.html
    /// [`PRAGMA synchronous`]: https://www.sqlite.org/pragma.html#pragma_synchronous
    pub fn synchronous(mut self, synchronous: Synchronous) -> Self {
        self.runtime_config.synchronous = Some(synchronous);
        self
    }

    /// Returns the pool configuration.
    pub(crate) fn pool_config(&self) -> PoolConfig {
        self.pool_config
    }

    /// Returns the runtime configuration.
    pub(crate) fn runtime_config(&self) -> RuntimeConfig {
        self.runtime_config
    }

    /// Returns how the store should encrypt and serialize what it writes.
    ///
    /// A [`Self::cipher_provider`] set explicitly wins over the secret set by
    /// [`Self::passphrase`] or [`Self::key`]; without either, the store stays
    /// unencrypted.
    pub(crate) fn encryption_config(&self) -> EncryptionConfig {
        let mut config = self.encryption.clone();

        if config.cipher_provider.is_none() {
            config.cipher_provider = self.secret.clone().map(|secret| {
                let provider: Arc<dyn StoreCipherProvider> =
                    Arc::new(SecretStoreCipherProvider::new(secret));
                provider
            });
        }

        config
    }

    /// Build a pool of active connections to a particular database.
    pub fn build_pool_of_connections(
        &self,
        database_name: &str,
    ) -> Result<connection::Pool, connection::CreatePoolError> {
        let path = self.path.join(database_name);
        let manager = connection::Manager::new(path);

        connection::Pool::builder(manager)
            .config(self.pool_config)
            .build()
            .map_err(connection::CreatePoolError::Build)
    }
}

/// This type represents values to set at runtime when a database is opened.
///
/// This configuration is applied by
/// [`utils::SqliteAsyncConnExt::apply_runtime_config`].
#[derive(Clone, Copy, Debug)]
struct RuntimeConfig {
    /// If `true`, [`utils::SqliteAsyncConnExt::optimize`] will be called.
    optimize: bool,

    /// Regardless of the value, [`utils::SqliteAsyncConnExt::cache_size`] will
    /// always be called with this value.
    cache_size: u32,

    /// Regardless of the value,
    /// [`utils::SqliteAsyncConnExt::journal_size_limit`] will always be called
    /// with this value.
    journal_size_limit: u32,

    /// If `Some`, [`utils::SqliteAsyncConnExt::synchronous`] will be called
    /// with this value. If `None`, each store applies its own default; see
    /// [`SqliteStoreConfig::synchronous`].
    synchronous: Option<Synchronous>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            // Optimize is always applied.
            optimize: true,
            // A cache of 2Mib.
            cache_size: 2_000_000,
            // A limit of 10Mib.
            journal_size_limit: 10_000_000,
            // No override; each store picks its own default.
            synchronous: None,
        }
    }
}

/// The value of [`PRAGMA synchronous`][pragma], controlling how aggressively
/// SQLite flushes data to disk.
///
/// [pragma]: https://www.sqlite.org/pragma.html#pragma_synchronous
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Synchronous {
    /// `PRAGMA synchronous = OFF`. SQLite continues without syncing as soon
    /// as it has handed data off to the operating system. This is the
    /// fastest option, but the database can be corrupted if the
    /// application crashes during a transaction, or on a power loss or OS
    /// crash.
    Off,

    /// `PRAGMA synchronous = NORMAL`. In [WAL mode][wal], the database is
    /// synced at checkpoints rather than on every commit. This is a lot
    /// cheaper than [`Synchronous::Full`], and the database cannot be
    /// corrupted, though a transaction can be rolled back after a power
    /// loss or an OS crash (not after an application crash).
    ///
    /// [wal]: https://www.sqlite.org/wal.html
    Normal,

    /// `PRAGMA synchronous = FULL`. The database engine syncs at every
    /// commit, guaranteeing that the database is durable across an
    /// application crash, an OS crash, or a power loss, at the cost of an
    /// `fsync` on every commit.
    Full,

    /// `PRAGMA synchronous = EXTRA`. Like [`Synchronous::Full`], but with
    /// an additional sync after data is sent to disk before the WAL file
    /// is checkpointed, for a small additional cost.
    Extra,
}

impl Synchronous {
    /// Returns the associated `PRAGMA synchronous` value.
    pub(crate) fn as_pragma_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
            Self::Extra => "EXTRA",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ops::Not,
        path::{Path, PathBuf},
    };

    use super::{POOL_MINIMUM_SIZE, Secret, SqliteStoreConfig, Synchronous};

    #[test]
    fn test_new() {
        let store_config = SqliteStoreConfig::new(Path::new("foo"));

        assert_eq!(store_config.pool_config.max_size, num_cpus::get_physical() * 4);
        assert!(store_config.runtime_config.optimize);
        assert_eq!(store_config.runtime_config.cache_size, 2_000_000);
        assert_eq!(store_config.runtime_config.journal_size_limit, 10_000_000);
        assert_eq!(store_config.runtime_config.synchronous, None);
    }

    #[test]
    fn test_with_low_memory_config() {
        let store_config = SqliteStoreConfig::with_low_memory_config(Path::new("foo"));

        assert_eq!(store_config.pool_config.max_size, num_cpus::get_physical());
        assert!(store_config.runtime_config.optimize);
        assert_eq!(store_config.runtime_config.cache_size, 500_000);
        assert_eq!(store_config.runtime_config.journal_size_limit, 2_000_000);
    }

    #[test]
    fn test_store_config_when_passphrase() {
        let store_config = SqliteStoreConfig::new(Path::new("foo"))
            .passphrase(Some("bar"))
            .pool_max_size(42)
            .optimize(false)
            .cache_size(43)
            .journal_size_limit(44)
            .synchronous(Synchronous::Full);

        assert_eq!(store_config.path, PathBuf::from("foo"));
        assert_eq!(store_config.secret, Some(Secret::PassPhrase("bar".to_owned().into())));
        assert_eq!(store_config.pool_config.max_size, 42);
        assert!(store_config.runtime_config.optimize.not());
        assert_eq!(store_config.runtime_config.cache_size, 43);
        assert_eq!(store_config.runtime_config.journal_size_limit, 44);
        assert_eq!(store_config.runtime_config.synchronous, Some(Synchronous::Full));
    }

    #[test]
    fn test_store_config_when_key() {
        let store_config = SqliteStoreConfig::new(Path::new("foo"))
            .key(Some(&[
                143, 27, 202, 78, 96, 55, 13, 149, 247, 8, 33, 120, 204, 92, 171, 66, 19, 238, 61,
                107, 132, 211, 40, 244, 71, 190, 99, 14, 173, 225, 6, 156,
            ]))
            .pool_max_size(42)
            .optimize(false)
            .cache_size(43)
            .journal_size_limit(44)
            .synchronous(Synchronous::Off);

        assert_eq!(store_config.path, PathBuf::from("foo"));
        assert_eq!(
            store_config.secret,
            Some(Secret::Key(Box::new([
                143, 27, 202, 78, 96, 55, 13, 149, 247, 8, 33, 120, 204, 92, 171, 66, 19, 238, 61,
                107, 132, 211, 40, 244, 71, 190, 99, 14, 173, 225, 6, 156,
            ])))
        );
        assert_eq!(store_config.pool_config.max_size, 42);
        assert!(store_config.runtime_config.optimize.not());
        assert_eq!(store_config.runtime_config.cache_size, 43);
        assert_eq!(store_config.runtime_config.journal_size_limit, 44);
        assert_eq!(store_config.runtime_config.synchronous, Some(Synchronous::Off));
    }

    #[test]
    fn test_store_config_path() {
        let store_config = SqliteStoreConfig::new(Path::new("foo")).path(Path::new("bar"));

        assert_eq!(store_config.path, PathBuf::from("bar"));
    }

    #[test]
    fn test_store_config_synchronous() {
        let store_config = SqliteStoreConfig::new(Path::new("foo")).synchronous(Synchronous::Off);

        assert_eq!(store_config.runtime_config.synchronous, Some(Synchronous::Off));
    }

    #[test]
    fn test_pool_size_has_a_minimum() {
        let store_config = SqliteStoreConfig::new(Path::new("foo")).pool_max_size(1);

        assert_eq!(store_config.pool_config.max_size, POOL_MINIMUM_SIZE);
    }
}
