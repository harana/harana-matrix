// Copyright 2026 The Harana Contributors
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

//! Errors from encoding and decoding records.

use std::fmt::Display;

/// The result of a codec operation.
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// A record that could not be encoded or decoded.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A value could not be encoded.
    #[error("failed to serialize: {0}")]
    SerdeSer(Box<str>),

    /// Bytes could not be decoded into the requested type.
    #[error("failed to deserialize: {0}")]
    SerdeDe(Box<str>),

    /// The output buffer could not be written to.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A `Json` payload could not be encoded or decoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// A fixed-width integer's bytes did not fill their buffer.
    ///
    /// The record ended mid-number, which means the stored bytes are damaged.
    #[error("integer buffer underflow")]
    Capacity,

    /// A length calculation went out of range.
    ///
    /// Decoding compares a cursor against a buffer's length, so this means the
    /// cursor ran past its own buffer.
    #[error("arithmetic overflow")]
    Arithmetic,
}

impl<T> From<arrayvec::CapacityError<T>> for Error {
    fn from(_: arrayvec::CapacityError<T>) -> Self {
        Self::Capacity
    }
}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::SerdeSer(msg.to_string().into())
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::SerdeDe(msg.to_string().into())
    }
}

/// The name of a type, for a debug assertion that inspects one.
#[inline]
#[must_use]
pub(crate) fn type_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
}

/// Marks a branch of the Serde data model this codec does not encode.
///
/// The compact record format covers what a key or value in a key-value store
/// needs, and nothing more: a caller reaching one of these branches has asked
/// for a shape the format cannot represent, and wants
/// [`crate::store_codec::Json`] around that value instead.
#[macro_export]
macro_rules! unhandled {
    ($msg:literal) => {
        unimplemented!($msg)
    };
}
