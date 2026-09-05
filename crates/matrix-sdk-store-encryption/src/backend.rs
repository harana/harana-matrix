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

//! A pluggable abstraction over the cipher a key/value store encrypts itself
//! with.
//!
//! Store implementations talk to a [`StoreCipherBackend`] rather than to a
//! concrete cipher, and obtain one from a [`StoreCipherProvider`]. The default
//! provider hands out [`StoreCipher`], which encrypts with XChaCha20-Poly1305
//! and keeps its key material in process memory; an application that would
//! rather hold key material in an OS keychain, a Secure Enclave, an HSM or a
//! KMS implements the two traits itself and installs its provider on the store
//! it opens.
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//!
//! use matrix_sdk_store_encryption::{
//!     CreatedStoreCipher, Error, StoreCipher, StoreCipherBackend,
//!     StoreCipherProvider,
//! };
//!
//! /// A provider that fetches the store key from somewhere outside the
//! /// database, so that no key material is ever written next to the data.
//! #[derive(Debug)]
//! struct KeychainProvider;
//!
//! impl KeychainProvider {
//!     fn key_from_the_keychain(&self) -> [u8; 32] {
//!         // … ask the platform keychain here …
//!         [0u8; 32]
//!     }
//! }
//!
//! impl StoreCipherProvider for KeychainProvider {
//!     fn import(
//!         &self,
//!         exported: &[u8],
//!     ) -> Result<Arc<dyn StoreCipherBackend>, Error> {
//!         let key = self.key_from_the_keychain();
//!         Ok(Arc::new(StoreCipher::import_with_key(&key, exported)?))
//!     }
//!
//!     fn create(&self) -> Result<CreatedStoreCipher, Error> {
//!         let key = self.key_from_the_keychain();
//!         let cipher = StoreCipher::new()?;
//!         let export = cipher.export_with_key(&key)?;
//!
//!         Ok((Arc::new(cipher), Some(export)))
//!     }
//! }
//! ```

use std::{fmt, sync::Arc};

use crate::{EncryptableValue, EncryptedValue, EncryptedValueBase64, Error, StoreCipher};

/// A freshly created cipher, plus the blob the store should persist so that
/// [`StoreCipherProvider::import`] can restore it later.
///
/// The blob is `None` for a provider that keeps its key material entirely
/// outside the database.
pub type CreatedStoreCipher = (Arc<dyn StoreCipherBackend>, Option<Vec<u8>>);

/// The cryptographic operations a key/value store needs from its cipher.
///
/// This is the object-safe counterpart of the inherent methods on
/// [`StoreCipher`], which is also the default implementation. Store backends
/// hold a `dyn StoreCipherBackend` so that the cipher can be replaced without
/// touching the storage code.
///
/// Implementations must be usable from several threads at once: stores call
/// into them from their connection pools.
pub trait StoreCipherBackend: fmt::Debug + Send + Sync {
    /// Hash a key before it is inserted into the key/value store.
    ///
    /// This prevents key names from leaking to parties that cannot decrypt the
    /// store. The transformation is one-way, and must be deterministic: the
    /// same `table_name` and `key` always produce the same hash, otherwise
    /// previously written rows become unreachable.
    ///
    /// `table_name` scopes the hash to one table, so that the same key in two
    /// tables hashes differently.
    fn hash_key(&self, table_name: &str, key: &[u8]) -> [u8; 32];

    /// Encrypt some data before it is inserted into the key/value store.
    ///
    /// Implementations should zeroize `data` once it has been encrypted, by
    /// calling [`EncryptableValue::zeroiize`].
    fn encrypt_value_data(&self, data: &mut dyn EncryptableValue) -> Result<EncryptedValue, Error>;

    /// Decrypt some data that was fetched from the key/value store.
    fn decrypt_value_data(&self, value: EncryptedValue) -> Result<Vec<u8>, Error>;

    /// Encrypt some data, encoding the byte arrays of the result as base64.
    ///
    /// The default implementation defers to [`Self::encrypt_value_data`].
    fn encrypt_value_base64_data(&self, mut data: Vec<u8>) -> Result<EncryptedValueBase64, Error> {
        self.encrypt_value_data(&mut data).map(EncryptedValueBase64::from)
    }

    /// Decrypt some data whose byte arrays are encoded as base64.
    ///
    /// The default implementation defers to [`Self::decrypt_value_data`].
    fn decrypt_value_base64_data(&self, value: EncryptedValueBase64) -> Result<Vec<u8>, Error> {
        self.decrypt_value_data(value.try_into()?)
    }
}

impl StoreCipherBackend for StoreCipher {
    fn hash_key(&self, table_name: &str, key: &[u8]) -> [u8; 32] {
        StoreCipher::hash_key(self, table_name, key)
    }

    fn encrypt_value_data(&self, data: &mut dyn EncryptableValue) -> Result<EncryptedValue, Error> {
        self.encrypt_value_bytes(data)
    }

    fn decrypt_value_data(&self, value: EncryptedValue) -> Result<Vec<u8>, Error> {
        StoreCipher::decrypt_value_data(self, value)
    }
}

/// Source of the [`StoreCipherBackend`] a store encrypts itself with.
///
/// A store calls [`StoreCipherProvider::import`] when it finds a previously
/// persisted cipher export, and [`StoreCipherProvider::create`] when it does
/// not. Whatever `create` returns as its second element is persisted verbatim
/// and handed back to `import` the next time the store is opened.
pub trait StoreCipherProvider: fmt::Debug + Send + Sync {
    /// Restore the cipher from an export previously returned by
    /// [`StoreCipherProvider::create`].
    fn import(&self, exported: &[u8]) -> Result<Arc<dyn StoreCipherBackend>, Error>;

    /// Create a fresh cipher for a store that does not have one yet.
    ///
    /// Returns the cipher, plus the blob the store should persist alongside
    /// its data so that [`StoreCipherProvider::import`] can restore the same
    /// cipher later.
    ///
    /// Providers that keep their key material entirely outside the database
    /// (in an OS keychain, say) return `None` instead of a blob. Nothing is
    /// then persisted, so the store calls `create` again on every open, and
    /// the provider is responsible for returning the same key each time.
    fn create(&self) -> Result<CreatedStoreCipher, Error>;
}
