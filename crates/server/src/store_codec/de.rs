// Copyright 2025 Tuwunel Contributors
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
//
// Ported from tuwunel `src/database/de.rs`. The CBOR wrapper is not ported,
// so a payload that needs a self-describing format uses `Json`.

//! Deserialization for the database's compact record codec.
//!
//! Compound values are divided into records by [`crate::store_codec::SEP`]. A
//! trailing type that accepts empty input can decode a missing final field from
//! an older, shorter tuple, while writing the extended tuple emits an
//! additional separator before that appended field.

use arrayvec::ArrayVec;
use serde::{
    Deserialize, de,
    de::{DeserializeSeed, Visitor},
};

use crate::{
    store_codec::{Error, Result},
    unhandled,
};

/// Deserializes a value from database record bytes.
///
/// The result may borrow from `buf` according to `T`'s deserialization
/// implementation. Debug builds additionally verify that decoding consumed the
/// input, apart from one trailing record separator.
///
/// # Panics
///
/// Panics if `T` requests a Serde data-model operation unsupported by this
/// codec. In debug builds, decoding also panics when record-layout invariants
/// are violated or unexpected trailing bytes remain.
pub fn from_slice<'a, T>(buf: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer { buf, pos: 0, rec: 0, seq: 0 };

    let value = T::deserialize(&mut deserializer)?;

    debug_assert!(
        deserializer.finished().is_ok(),
        "deserialization failed to consume trailing bytes"
    );

    Ok(value)
}

/// Cursor state for decoding the compact database record format.
///
/// The byte position and record counters advance independently so incomplete
/// tuple tails can be presented to inner deserializers as empty input.
/// `next_element_seed` compares the record count with the expected sequence
/// length when deciding whether iteration is complete.
pub(crate) struct Deserializer<'de> {
    buf: &'de [u8],
    pos: usize,
    rec: usize,
    seq: usize,
}

/// Skips one encoded record when deserialized inside a sequence.
///
/// At the top level, the directive consumes all remaining records so the input
/// finishes cleanly. It produces only the unit value represented by this
/// marker.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Ignore;

/// Skips the current record and every record that follows it.
///
/// Place this directive inside a sequence to discard its remaining encoded
/// elements. It consumes the trailing input and produces only the unit value
/// represented by this marker.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct IgnoreAll;

impl<'de> Deserializer<'de> {
    const SEP: u8 = crate::store_codec::ser::SEP;

    /// Determine if the input was fully consumed and error if bytes remaining.
    /// This is intended for debug assertions; not optimized for parsing logic.
    fn finished(&self) -> Result {
        let pos = self.pos;
        let len = self.buf.len();
        let parsed = &self.buf[0..pos];
        let unparsed = &self.buf[pos..];
        let remain = self.remaining()?;
        let trailing_sep = remain == 1 && unparsed[0] == Self::SEP;
        (remain == 0 || trailing_sep).then_some(()).ok_or_else(|| {
            Error::SerdeDe(
                format!(
                    "{remain} trailing of {len} bytes not deserialized.\n{parsed:?}\n{unparsed:?}"
                )
                .into(),
            )
        })
    }

    /// Called at the start of arrays and tuples
    #[inline]
    fn sequence_start(&mut self, len: usize) {
        debug_assert!(self.seq == 0, "Nested sequences are not handled at this time");
        self.seq = len;
    }

    /// Consume the current record to ignore it. Inside a sequence the next
    /// record is skipped but at the top-level all records are skipped such that
    /// deserialization completes with self.finished() == Ok.
    #[inline]
    fn record_ignore(&mut self) {
        if self.seq > 0 {
            self.record_next();
        } else {
            self.record_ignore_all();
        }
    }

    /// Consume the current and all remaining records to ignore them. Similar to
    /// Ignore at the top-level, but it can be provided in a sequence to Ignore
    /// all remaining elements.
    #[inline]
    fn record_ignore_all(&mut self) {
        self.record_trail();
    }

    /// Consume the current record. The position pointer is moved to the start
    /// of the next record. Slice of the current record is returned.
    #[inline]
    fn record_next(&mut self) -> &'de [u8] {
        self.buf[self.pos..]
            .split(|b| *b == Deserializer::SEP)
            .inspect(|record| self.inc_pos(record.len()))
            .next()
            .expect("remainder of buf even if SEP was not found")
    }

    /// Peek at the first byte of the current record. If all records were
    /// consumed None is returned instead.
    #[inline]
    fn record_peek_byte(&self) -> Option<u8> {
        let started = self.pos != 0 || self.rec > 0;
        let buf = &self.buf[self.pos..];
        debug_assert!(
            !started || buf[0] == Self::SEP,
            "Missing expected record separator at current position"
        );

        buf.get::<usize>(started.into()).copied()
    }

    /// Consume the record separator such that the position cleanly points to
    /// the start of the next record. When input is exhausted but the
    /// sequence is not, advance no bytes; the caller deserializes from an
    /// empty slice. See `next_element_seed` for the additive-tail mechanic
    /// this enables.
    #[inline]
    fn record_start(&mut self) {
        let started = self.pos != 0 || self.rec > 0;
        let input_done = self.pos >= self.buf.len();
        let output_done = self.rec >= self.seq;
        let incomplete = input_done && !output_done;
        debug_assert!(
            !started || incomplete || self.buf.get(self.pos) == Some(&Self::SEP),
            "Missing expected record separator at current position"
        );

        let inc = started && !incomplete;
        self.inc_pos(inc.into());
        self.inc_rec(1);
    }

    /// Consume all remaining bytes, which may include record separators,
    /// returning a raw slice.
    #[inline]
    fn record_trail(&mut self) -> &'de [u8] {
        let record = &self.buf[self.pos..];
        self.inc_pos(record.len());
        record
    }

    /// Increment the position pointer.
    #[inline]
    fn inc_pos(&mut self, n: usize) {
        self.pos = self.pos.saturating_add(n);
        debug_assert!(self.pos <= self.buf.len(), "pos out of range");
    }

    #[inline]
    fn inc_rec(&mut self, n: usize) {
        self.rec = self.rec.saturating_add(n);
    }

    /// Unconsumed input bytes.
    #[inline]
    fn remaining(&self) -> Result<usize> {
        let pos = self.pos;
        let len = self.buf.len();
        len.checked_sub(pos).ok_or(Error::Arithmetic)
    }
}

impl<'a, 'de: 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.sequence_start(1);
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.sequence_start(len);
        visitor.visit_seq(self)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.sequence_start(len);
        visitor.visit_seq(self)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let input = self.record_next();
        let mut d = serde_json::Deserializer::from_slice(input);
        d.deserialize_map(visitor).map_err(Into::into)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let input = self.record_next();
        let mut d = serde_json::Deserializer::from_slice(input);
        d.deserialize_struct(name, fields, visitor).map_err(Into::into)
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match name {
            "Ignore" => self.record_ignore(),
            "IgnoreAll" => self.record_ignore_all(),
            _ => unhandled!("Unrecognized deserialization Directive {name:?}"),
        }

        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match name {
            "$serde_json::private::RawValue" => visitor.visit_map(self),
            "Json" => visitor
                .visit_newtype_struct(&mut serde_json::Deserializer::from_slice(
                    self.record_trail(),
                ))
                .map_err(|e| Self::Error::SerdeDe(format!("{name}: {e}").into())),

            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unhandled!("deserialize Enum not implemented")
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.buf.get(self.pos).is_none_or(|b| *b == Deserializer::SEP) {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize bool not implemented")
    }

    fn deserialize_i8<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize i8 not implemented")
    }

    fn deserialize_i16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize i16 not implemented")
    }

    fn deserialize_i32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize i32 not implemented")
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        const BYTES: usize = size_of::<i64>();

        let end = self.pos.saturating_add(BYTES).min(self.buf.len());
        let bytes: ArrayVec<u8, BYTES> = self.buf[self.pos..end].try_into()?;
        let bytes =
            bytes.into_inner().map_err(|_| Self::Error::SerdeDe("i64 buffer underflow".into()))?;

        self.inc_pos(BYTES);
        visitor.visit_i64(i64::from_be_bytes(bytes))
    }

    fn deserialize_u8<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!(
            "deserialize u8 not implemented; try dereferencing the Handle for [u8] access \
			 instead"
        )
    }

    fn deserialize_u16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize u16 not implemented")
    }

    fn deserialize_u32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize u32 not implemented")
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        const BYTES: usize = size_of::<u64>();

        let end = self.pos.saturating_add(BYTES).min(self.buf.len());
        let bytes: ArrayVec<u8, BYTES> = self.buf[self.pos..end].try_into()?;
        let bytes =
            bytes.into_inner().map_err(|_| Self::Error::SerdeDe("u64 buffer underflow".into()))?;

        self.inc_pos(BYTES);
        visitor.visit_u64(u64::from_be_bytes(bytes))
    }

    fn deserialize_f32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize f32 not implemented")
    }

    fn deserialize_f64<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize f64 not implemented")
    }

    fn deserialize_char<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize char not implemented")
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let input = self.record_next();
        let out = deserialize_str(input)?;
        visitor.visit_borrowed_str(out)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let input = self.record_next();
        let out = str_from_bytes(input)?.to_owned();
        visitor.visit_string(out)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let input = self.record_trail();
        visitor.visit_borrowed_bytes(input)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize Byte Buf not implemented")
    }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize Unit not implemented")
    }

    // this only used for $serde_json::private::RawValue at this time; see MapAccess
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let input = "$serde_json::private::RawValue";
        visitor.visit_borrowed_str(input)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        unhandled!("deserialize Ignored Any not implemented")
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        const TYPE_PRE_1_91: &str = "serde_json::value::de::<impl serde_core::de::Deserialize \
		                             for serde_json::value::Value>::deserialize::ValueVisitor";
        const TYPE: &str = "serde_json::value::de::<impl serde_core::de::Deserialize<'_> for \
		                    serde_json::value::Value>::deserialize::ValueVisitor";
        debug_assert!(
            matches!(crate::store_codec::error::type_name::<V>(), TYPE | TYPE_PRE_1_91),
            "deserialize_any: type not expected {0}",
            crate::store_codec::error::type_name::<V>()
        );

        match self.record_peek_byte() {
            Some(b'{') => self.deserialize_map(visitor),
            Some(b'[') => serde_json::Deserializer::from_slice(self.record_next())
                .deserialize_seq(visitor)
                .map_err(Into::into),

            _ => self.deserialize_str(visitor),
        }
    }
}

impl<'a, 'de: 'a> de::SeqAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        // Finished parsing the input.
        let finished = self.pos >= self.buf.len();

        // Completely satisfied the output.
        let complete = self.rec >= self.seq;

        // Early-return only when both input and output are exhausted. If
        // input is exhausted but the tuple is not, fall through:
        // `record_start` does not advance pos and the inner deserializer
        // runs against an empty slice. Tail types that visit an empty slice
        // successfully (`&str` -> "", `&[u8]` -> &[], `Option<_>` -> None)
        // round-trip; non-tolerant tails (numerics, typed Matrix IDs) error.
        // This enables additive evolution of record-key tuples without a
        // migration.
        //
        // Returning early before the input is exhausted trips the
        // `finished()` check; before the tuple is exhausted, serde's length
        // check.
        if finished && complete {
            return Ok(None);
        }

        self.record_start();
        seed.deserialize(&mut **self).map(Some)
    }
}

// this only used for $serde_json::private::RawValue at this time. our db
// schema doesn't have its own map format; we use json for that anyway
impl<'a, 'de: 'a> de::MapAccess<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut **self).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut **self)
    }
}

#[inline]
fn deserialize_str(input: &[u8]) -> Result<&str> {
    str_from_bytes(input)
}

/// Borrows a byte slice as UTF-8 text.
///
/// Strings were written by the serializer, so invalid UTF-8 here means the
/// stored bytes are damaged.
#[inline]
fn str_from_bytes(bytes: &[u8]) -> Result<&str> {
    std::str::from_utf8(bytes).map_err(|error| Error::SerdeDe(error.to_string().into()))
}
