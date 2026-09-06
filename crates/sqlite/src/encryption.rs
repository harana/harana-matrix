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

//! How an SQLite store encrypts and serializes what it writes.
//!
//! Both halves are pluggable: the cipher comes from a
//! [`StoreCipherProvider`], and the on-disk format from a pair of
//! [`StoreCodec`]s. The defaults reproduce what the store has always done —
//! XChaCha20-Poly1305 keyed by a passphrase or a raw key, MessagePack for
//! opaque values and JSON for Matrix payloads — so a store opened without
//! overriding either keeps reading and writing exactly the same bytes.

use std::{fmt, sync::Arc};

use store_encryption::{
    CreatedStoreCipher, Error as EncryptionError, JsonCodec, MessagePackCodec, StoreCipher,
    StoreCipherBackend, StoreCipherProvider, StoreCodec,
};

use crate::Secret;

/// The [`StoreCipherProvider`] the SQLite stores use by default: a
/// [`StoreCipher`] unlocked with a passphrase or a raw key, whose encrypted
/// export lives in the database next to the data.
#[derive(Clone)]
pub struct SecretStoreCipherProvider {
    secret: Secret,
}

impl SecretStoreCipherProvider {
    /// Build a provider unlocking the store with the given secret.
    pub fn new(secret: Secret) -> Self {
        Self { secret }
    }
}

impl fmt::Debug for SecretStoreCipherProvider {
    /// Never print the secret.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.secret {
            Secret::Key(_) => "key",
            Secret::PassPhrase(_) => "passphrase",
        };

        f.debug_struct("SecretStoreCipherProvider").field("secret", &kind).finish()
    }
}

impl StoreCipherProvider for SecretStoreCipherProvider {
    fn import(&self, exported: &[u8]) -> Result<Arc<dyn StoreCipherBackend>, EncryptionError> {
        Ok(match &self.secret {
            Secret::PassPhrase(passphrase) => Arc::new(StoreCipher::import(passphrase, exported)?),
            Secret::Key(key) => Arc::new(StoreCipher::import_with_key(key.as_slice(), exported)?),
        })
    }

    fn create(&self) -> Result<CreatedStoreCipher, EncryptionError> {
        let cipher = StoreCipher::new()?;

        let export = match &self.secret {
            Secret::PassPhrase(passphrase) => {
                #[cfg(not(test))]
                {
                    cipher.export(passphrase)
                }
                #[cfg(test)]
                {
                    cipher._insecure_export_fast_for_testing(passphrase)
                }
            }
            Secret::Key(key) => cipher.export_with_key(key.as_slice()),
        }?;

        Ok((Arc::new(cipher), Some(export)))
    }
}

/// How a store should encrypt and serialize its data, before it has been
/// opened.
///
/// Derived from a [`SqliteStoreConfig`](crate::SqliteStoreConfig) and turned
/// into a [`StoreEncryption`] once a connection is available, by
/// [`SqliteKeyValueStoreAsyncConnExt::open_store_encryption`].
///
/// [`SqliteKeyValueStoreAsyncConnExt::open_store_encryption`]: crate::utils::SqliteKeyValueStoreAsyncConnExt::open_store_encryption
#[derive(Clone, Debug)]
pub(crate) struct EncryptionConfig {
    /// Where the store's cipher comes from. `None` leaves the store
    /// unencrypted.
    pub cipher_provider: Option<Arc<dyn StoreCipherProvider>>,

    /// The format opaque values are written in.
    pub value_codec: Arc<dyn StoreCodec>,

    /// The format Matrix payloads are written in.
    pub json_codec: Arc<dyn StoreCodec>,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            cipher_provider: None,
            value_codec: Arc::new(MessagePackCodec),
            json_codec: Arc::new(JsonCodec),
        }
    }
}

/// How an open store encrypts and serializes its data.
///
/// The opened counterpart of [`EncryptionConfig`]: the cipher provider has
/// been asked for a cipher, so this holds the cipher itself.
#[derive(Clone)]
pub(crate) struct StoreEncryption {
    cipher: Option<Arc<dyn StoreCipherBackend>>,
    value_codec: Arc<dyn StoreCodec>,
    json_codec: Arc<dyn StoreCodec>,
}

impl StoreEncryption {
    pub(crate) fn new(
        cipher: Option<Arc<dyn StoreCipherBackend>>,
        config: &EncryptionConfig,
    ) -> Self {
        Self {
            cipher,
            value_codec: config.value_codec.clone(),
            json_codec: config.json_codec.clone(),
        }
    }

    /// The cipher the store encrypts keys and values with, if it is encrypted
    /// at all.
    pub(crate) fn cipher(&self) -> Option<&dyn StoreCipherBackend> {
        self.cipher.as_deref()
    }

    /// The codec for opaque values, and for the envelope encrypted values are
    /// wrapped in.
    pub(crate) fn value_codec(&self) -> &dyn StoreCodec {
        self.value_codec.as_ref()
    }

    /// The codec for Matrix payloads.
    pub(crate) fn json_codec(&self) -> &dyn StoreCodec {
        self.json_codec.as_ref()
    }
}

impl fmt::Debug for StoreEncryption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoreEncryption")
            .field("encrypted", &self.cipher.is_some())
            .field("value_codec", &self.value_codec.name())
            .field("json_codec", &self.json_codec.name())
            .finish()
    }
}

#[cfg(all(test, feature = "state-store"))]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use base::{StateStore, StateStoreDataKey, StateStoreDataValue};
    use sdk_test::async_test;
    use store_encryption::{
        CodecError, CreatedStoreCipher, JsonCodec, StoreCipher, StoreCipherBackend,
        StoreCipherProvider, StoreCodec,
    };
    use tempfile::tempdir;

    use super::*;
    use crate::{SqliteStateStore, SqliteStoreConfig};

    /// A provider holding its key outside the database, as an app backed by an
    /// OS keychain would. It persists no export, so the store asks it to
    /// create the cipher again on every open.
    #[derive(Debug)]
    struct ExternalKeyProvider {
        key: [u8; 32],
        creates: Arc<AtomicUsize>,
    }

    impl StoreCipherProvider for ExternalKeyProvider {
        fn import(&self, exported: &[u8]) -> Result<Arc<dyn StoreCipherBackend>, EncryptionError> {
            Ok(Arc::new(StoreCipher::import_with_key(&self.key, exported)?))
        }

        fn create(&self) -> Result<CreatedStoreCipher, EncryptionError> {
            self.creates.fetch_add(1, Ordering::SeqCst);

            // Derive the same cipher every time from the externally held key,
            // and keep nothing in the database.
            let cipher = StoreCipher::import_with_key(&self.key, &SEALED_CIPHER)?;

            Ok((Arc::new(cipher), None))
        }
    }

    /// A cipher export sealed with `[7u8; 32]`, generated once so that
    /// `ExternalKeyProvider` hands out a stable cipher across opens.
    static SEALED_CIPHER: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
        StoreCipher::new().unwrap().export_with_key(&[7u8; 32]).unwrap()
    });

    /// A codec that counts how often it is used, wrapping JSON.
    #[derive(Debug, Default)]
    struct CountingJsonCodec {
        encodes: AtomicUsize,
        decodes: AtomicUsize,
    }

    impl StoreCodec for CountingJsonCodec {
        fn name(&self) -> &'static str {
            "counting-json"
        }

        fn encode(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, CodecError> {
            self.encodes.fetch_add(1, Ordering::SeqCst);
            JsonCodec.encode(value)
        }

        fn with_deserializer<'de>(
            &self,
            bytes: &'de [u8],
            visit: &mut dyn FnMut(
                &mut dyn erased_serde::Deserializer<'de>,
            ) -> Result<(), CodecError>,
        ) -> Result<(), CodecError> {
            self.decodes.fetch_add(1, Ordering::SeqCst);
            JsonCodec.with_deserializer(bytes, visit)
        }
    }

    #[async_test]
    async fn test_custom_cipher_provider_and_codec_round_trip() {
        let dir = tempdir().unwrap();
        let creates = Arc::new(AtomicUsize::new(0));
        let codec = Arc::new(CountingJsonCodec::default());

        let config = |creates: &Arc<AtomicUsize>, codec: &Arc<CountingJsonCodec>| {
            SqliteStoreConfig::new(dir.path())
                .cipher_provider(Some(Arc::new(ExternalKeyProvider {
                    key: [7u8; 32],
                    creates: creates.clone(),
                })))
                .value_codec(codec.clone())
        };

        let store = SqliteStateStore::open_with_config(&config(&creates, &codec)).await.unwrap();
        store
            .set_kv_data(
                StateStoreDataKey::SyncToken,
                StateStoreDataValue::SyncToken("a-token".to_owned()),
            )
            .await
            .unwrap();

        // The custom codec, not MessagePack, wrote the value.
        assert!(codec.encodes.load(Ordering::SeqCst) > 0);

        // Reopening with the same externally held key reads the data back,
        // even though nothing about the cipher was persisted.
        drop(store);
        let store = SqliteStateStore::open_with_config(&config(&creates, &codec)).await.unwrap();

        let token = store
            .get_kv_data(StateStoreDataKey::SyncToken)
            .await
            .unwrap()
            .map(|value| value.into_sync_token().unwrap());
        assert_eq!(token.as_deref(), Some("a-token"));
        assert!(codec.decodes.load(Ordering::SeqCst) > 0);

        // Both opens went through `create`, since the provider persists no
        // export of its own.
        assert_eq!(creates.load(Ordering::SeqCst), 2);
    }

    #[async_test]
    async fn test_default_config_keeps_the_message_pack_codec() {
        let dir = tempdir().unwrap();
        let config = SqliteStoreConfig::new(dir.path()).passphrase(Some("secret"));
        let encryption = config.encryption_config();

        assert_eq!(encryption.value_codec.name(), "messagepack");
        assert_eq!(encryption.json_codec.name(), "json");
        assert!(encryption.cipher_provider.is_some());

        // Without a secret and without a provider, the store stays in the
        // clear.
        assert!(SqliteStoreConfig::new(dir.path()).encryption_config().cipher_provider.is_none());
    }
}
