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

//! A pluggable abstraction over the serialization format a store writes to
//! disk.
//!
//! Store implementations serialize their values through a [`StoreCodec`]
//! rather than calling a serialization crate directly. Two codecs ship with
//! the SDK: [`MessagePackCodec`], which is what the SQLite store has always
//! used for its opaque value columns, and [`JsonCodec`], used where the column
//! holds Matrix JSON. Applications that want a different format on disk
//! implement the trait and install their codec on the store they open.
//!
//! Swapping the codec of a store that already holds data makes that data
//! unreadable: the format is part of the on-disk layout, so a codec change
//! needs the same treatment as a schema migration.
//!
//! # Example
//!
//! ```
//! use matrix_sdk_store_encryption::{CodecError, StoreCodec};
//!
//! #[derive(Debug)]
//! struct CborCodec;
//!
//! impl StoreCodec for CborCodec {
//!     fn name(&self) -> &'static str {
//!         "cbor"
//!     }
//!
//!     fn encode(
//!         &self,
//!         value: &dyn erased_serde::Serialize,
//!     ) -> Result<Vec<u8>, CodecError> {
//!         # let _ = value;
//!         // … serialize with your CBOR crate of choice here …
//!         # Ok(Vec::new())
//!     }
//!
//!     fn with_deserializer<'de>(
//!         &self,
//!         bytes: &'de [u8],
//!         visit: &mut dyn FnMut(
//!             &mut dyn erased_serde::Deserializer<'de>,
//!         ) -> Result<(), CodecError>,
//!     ) -> Result<(), CodecError> {
//!         # let _ = (bytes, visit);
//!         // … and hand a deserializer over the same format to `visit` …
//!         # unimplemented!()
//!     }
//! }
//! ```

use std::{error::Error as StdError, fmt};

use serde::{Serialize, de::DeserializeOwned};

/// An error raised by a [`StoreCodec`] while encoding or decoding a value.
///
/// The underlying error is whatever the codec's serialization crate produced,
/// available through [`std::error::Error::source`].
#[derive(Debug)]
pub struct CodecError {
    codec: &'static str,
    source: Box<dyn StdError + Send + Sync + 'static>,
}

impl CodecError {
    /// Wrap the error a codec ran into.
    pub fn new(codec: &'static str, source: impl StdError + Send + Sync + 'static) -> Self {
        Self { codec, source: Box::new(source) }
    }

    /// The [`StoreCodec::name`] of the codec that raised this error.
    pub fn codec(&self) -> &'static str {
        self.codec
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the `{}` store codec failed: {}", self.codec, self.source)
    }
}

impl StdError for CodecError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}

/// The serialization format a store uses for the values it persists.
///
/// The trait is object-safe, so stores hold a `dyn StoreCodec`. The typed
/// helpers most callers want live on [`StoreCodecExt`], which is implemented
/// for every `StoreCodec`.
pub trait StoreCodec: fmt::Debug + Send + Sync {
    /// A short name for this codec, used in error messages.
    fn name(&self) -> &'static str;

    /// Serialize a value to the bytes that will be written to the store.
    fn encode(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, CodecError>;

    /// Build a deserializer over bytes fetched from the store and hand it to
    /// `visit`.
    ///
    /// Lending a deserializer, rather than returning a deserialized value,
    /// lets callers layer their own diagnostics on top: the SQLite store wraps
    /// it in `serde_path_to_error` to report which field of a stored value
    /// failed to parse. Most callers instead want the typed
    /// [`StoreCodecExt::decode_value`].
    ///
    /// Implementations must call `visit` exactly once, and propagate whatever
    /// it returns.
    fn with_deserializer<'de>(
        &self,
        bytes: &'de [u8],
        visit: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> Result<(), CodecError>,
    ) -> Result<(), CodecError>;
}

/// Typed helpers on top of [`StoreCodec`], implemented for every codec.
pub trait StoreCodecExt: StoreCodec {
    /// Serialize a value to the bytes that will be written to the store.
    fn encode_value<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        self.encode(value)
    }

    /// Deserialize a value that was fetched from the store.
    fn decode_value<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, CodecError> {
        let name = self.name();
        let mut decoded = None;

        self.with_deserializer(bytes, &mut |deserializer| {
            decoded = Some(
                erased_serde::deserialize::<T>(deserializer)
                    .map_err(|error| CodecError::new(name, error))?,
            );

            Ok(())
        })?;

        Ok(decoded.expect("a `StoreCodec` must call the visitor it is handed"))
    }
}

impl<C: StoreCodec + ?Sized> StoreCodecExt for C {}

/// The MessagePack codec, backed by [`rmp_serde`].
///
/// This is the format the SQLite store has always written its opaque value
/// columns in, and stays the default so that existing databases keep working.
/// Structs are encoded with named fields, matching `rmp_serde::to_vec_named`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MessagePackCodec;

impl StoreCodec for MessagePackCodec {
    fn name(&self) -> &'static str {
        "messagepack"
    }

    fn encode(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, CodecError> {
        let mut bytes = Vec::new();
        let mut serializer = rmp_serde::Serializer::new(&mut bytes).with_struct_map();

        erased_serde::serialize(value, &mut serializer)
            .map_err(|error| CodecError::new(self.name(), error))?;

        Ok(bytes)
    }

    fn with_deserializer<'de>(
        &self,
        bytes: &'de [u8],
        visit: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> Result<(), CodecError>,
    ) -> Result<(), CodecError> {
        let mut deserializer = rmp_serde::Deserializer::from_read_ref(bytes);
        let mut erased = <dyn erased_serde::Deserializer<'_>>::erase(&mut deserializer);

        visit(&mut erased)
    }
}

/// The JSON codec, backed by [`serde_json`].
///
/// Used where a stored value is Matrix JSON that other parts of the SDK, or
/// other clients sharing the database, need to be able to read.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonCodec;

impl StoreCodec for JsonCodec {
    fn name(&self) -> &'static str {
        "json"
    }

    fn encode(&self, value: &dyn erased_serde::Serialize) -> Result<Vec<u8>, CodecError> {
        let mut bytes = Vec::new();
        let mut serializer = serde_json::Serializer::new(&mut bytes);

        erased_serde::serialize(value, &mut serializer)
            .map_err(|error| CodecError::new(self.name(), error))?;

        Ok(bytes)
    }

    fn with_deserializer<'de>(
        &self,
        bytes: &'de [u8],
        visit: &mut dyn FnMut(&mut dyn erased_serde::Deserializer<'de>) -> Result<(), CodecError>,
    ) -> Result<(), CodecError> {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let mut erased = <dyn erased_serde::Deserializer<'_>>::erase(&mut deserializer);

        visit(&mut erased)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{JsonCodec, MessagePackCodec, StoreCodec, StoreCodecExt};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Value {
        name: String,
        count: u32,
    }

    fn round_trip(codec: &dyn StoreCodec) {
        let value = Value { name: "bulbasaur".to_owned(), count: 42 };

        let encoded = codec.encode_value(&value).unwrap();
        let decoded: Value = codec.decode_value(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_message_pack_round_trip() {
        round_trip(&MessagePackCodec);
    }

    #[test]
    fn test_json_round_trip() {
        round_trip(&JsonCodec);
    }

    #[test]
    fn test_message_pack_matches_to_vec_named() {
        // The default codec must stay byte-compatible with what the SQLite
        // store wrote before it went through a codec, otherwise existing
        // databases stop parsing.
        let value = Value { name: "bulbasaur".to_owned(), count: 42 };

        assert_eq!(
            MessagePackCodec.encode_value(&value).unwrap(),
            rmp_serde::to_vec_named(&value).unwrap(),
        );
    }

    #[test]
    fn test_decode_error_names_the_codec() {
        let error = MessagePackCodec.decode_value::<Value>(&[0xc1]).unwrap_err();

        assert_eq!(error.codec(), "messagepack");
    }
}
