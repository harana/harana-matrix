// Copyright 2021 David Briggs
// Copyright 2026 The Matrix.org Foundation C.I.C.
//
// Vendored from the `growable-bloom-filter` crate (MIT licensed), see
// https://github.com/dpbriggs/growable-bloom-filters/blob/master/src/stable_hasher.rs
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! A [`Hasher`] whose output is identical on every platform.

use std::{
    fmt,
    hash::{BuildHasher, Hasher},
};

/// Wrapper over a hasher that provides stable output across platforms.
///
/// Based on [rustc's `StableHasher`][rustc].
///
/// To that end we always convert integers to little-endian format before
/// hashing and the architecture dependent `isize` and `usize` types are
/// extended to 64 bits if needed.
///
/// The underlying hash function is xxh3-64 with the default seed and secret,
/// so the digest is stable across releases of this crate as well and can be
/// persisted.
///
/// # Example
///
/// ```rust
/// use std::hash::{Hash, Hasher};
///
/// use client_common::stable_hasher::StableHasher;
///
/// let mut hasher = StableHasher::new();
/// "hello world".hash(&mut hasher);
///
/// // The same value hashes to the same digest on a 32-bit big-endian target
/// // as it does on a 64-bit little-endian one.
/// assert_eq!(hasher.finish(), 1_312_102_073_844_821_397);
/// ```
///
/// [rustc]: https://github.com/rust-lang/rust/blob/c0955a34bcb17f0b31d7b86522a520ebe7fa93ac/src/librustc_data_structures/stable_hasher.rs#L78-L166
#[derive(Clone, Default)]
pub struct StableHasher {
    /// Using xxh3-64 with default seed/secret as the portable hasher.
    state: xxhash_rust::xxh3::Xxh3,
}

impl StableHasher {
    /// Create a new, empty [`StableHasher`].
    #[inline]
    pub fn new() -> Self {
        Self { state: xxhash_rust::xxh3::Xxh3::new() }
    }
}

impl fmt::Debug for StableHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `Xxh3` holds a large internal buffer and is not `Debug`, so only the
        // digest of the current state is of any use here.
        f.debug_struct("StableHasher").field("digest", &self.finish()).finish()
    }
}

impl Hasher for StableHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.state.finish()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.state.write(bytes);
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.state.write_u8(i);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.state.write_u16(i.to_le());
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.state.write_u32(i.to_le());
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.state.write_u64(i.to_le());
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.state.write_u128(i.to_le());
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        // Always treat usize as u64 so we get the same results on 32 and 64 bit
        // platforms. This is important for symbol hashes when cross compiling,
        // for example.
        self.state.write_u64((i as u64).to_le());
    }

    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.state.write_i8(i);
    }

    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.state.write_i16(i.to_le());
    }

    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.state.write_i32(i.to_le());
    }

    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.state.write_i64(i.to_le());
    }

    #[inline]
    fn write_i128(&mut self, i: i128) {
        self.state.write_i128(i.to_le());
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        // Always treat isize as i64 so we get the same results on 32 and 64 bit
        // platforms. This is important for symbol hashes when cross compiling,
        // for example.
        self.state.write_i64((i as i64).to_le());
    }
}

/// A [`BuildHasher`] that hands out [`StableHasher`]s.
///
/// Unlike [`std::hash::RandomState`] this is not randomly seeded, so it can be
/// used where hashes must stay identical across processes and platforms.
#[derive(Clone, Copy, Debug, Default)]
pub struct StableHasherBuilder;

impl BuildHasher for StableHasherBuilder {
    type Hasher = StableHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        StableHasher::new()
    }
}

#[cfg(test)]
mod tests {
    use std::hash::{BuildHasher, Hash, Hasher};

    use super::{StableHasher, StableHasherBuilder};

    /// Digests of a few values, recorded on a 64-bit little-endian platform.
    ///
    /// If any of these change, previously persisted hashes (for instance the
    /// bloom filters in the state store) stop matching.
    #[test]
    fn test_digests_are_stable() {
        fn digest<T: Hash>(value: T) -> u64 {
            let mut hasher = StableHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        assert_eq!(digest("hello world"), 1_312_102_073_844_821_397);
        assert_eq!(digest(0u8), 14_144_645_293_874_801_883);
        assert_eq!(digest(-1i64), 5_841_669_975_847_748_627);
        assert_eq!(digest(1_usize), 3_439_722_301_264_460_078);
        assert_eq!(digest(vec![1u32, 2, 3]), 16_389_153_945_020_288_093);
    }

    /// `usize`/`isize` must hash exactly like their 64-bit counterparts, so
    /// that 32-bit targets (wasm) agree with 64-bit ones.
    #[test]
    fn test_pointer_sized_integers_are_widened() {
        fn digest<T: Hash>(value: T) -> u64 {
            let mut hasher = StableHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        }

        assert_eq!(digest(42_usize), digest(42_u64));
        assert_eq!(digest(-42_isize), digest(-42_i64));
    }

    #[test]
    fn test_build_hasher_is_not_seeded() {
        let builder = StableHasherBuilder;

        assert_eq!(builder.hash_one("hello world"), builder.hash_one("hello world"));

        let mut hasher = StableHasher::new();
        "hello world".hash(&mut hasher);
        assert_eq!(builder.hash_one("hello world"), hasher.finish());
    }
}
