// Copyright 2021 David Briggs
// Copyright 2026 The Matrix.org Foundation C.I.C.
//
// Vendored from the `growable-bloom-filter` crate (MIT licensed), see
// https://github.com/dpbriggs/growable-bloom-filters
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

//! An implementation of [Scalable Bloom Filters].
//!
//! The serialized representation is byte-for-byte identical to the one of the
//! `growable-bloom-filter` crate this was vendored from, so filters persisted
//! by older versions of the SDK keep deserializing.
//!
//! [Scalable Bloom Filters]: https://gsd.di.uminho.pt/members/cbm/ps/dbloom.pdf

use std::{
    hash::{Hash, Hasher},
    num::NonZeroU64,
};

use serde::{Deserialize, Serialize};

use crate::stable_hasher::StableHasher;

/// Base Bloom Filter
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
struct Bloom {
    /// The actual bit field. Set to 0 with `Bloom::new`.
    #[serde(rename = "b", with = "serde_bytes")]
    buffer: Box<[u8]>,
    /// The number of slices in the partitioned bloom filter.
    /// Equivalent to the number of hash function in the classic bloom filter.
    /// An insertion will result in a bit being set in each slice.
    #[serde(rename = "k")]
    num_slices: NonZeroU64,
}

impl Bloom {
    /// Create a new Bloom filter (specifically, a Partitioned Bloom filter)
    ///
    /// # Arguments
    ///
    /// * `capacity` - target capacity. Panics if `capacity` is zero.
    /// * `error_ratio` - false positive ratio (0..1.0).
    fn new(capacity: usize, error_ratio: f64) -> Bloom {
        // Directly from paper:
        // k = log2(1/P)   (num_slices)
        // n ~= -m ln(1-p)  (slice_len_bits)
        // M = k * m       (total_bits)
        // for optimal filter p = 0.5, which gives:
        // n ~= -m ln(0.5), rearranging: m = -n / ln(0.5) = n / ln(2)
        debug_assert!(capacity >= 1);
        debug_assert!(0.0 < error_ratio && error_ratio < 1.0);
        // We're using ceil instead of round in order to get an error rate <= the
        // desired. Using round can result in significantly higher error
        // rates.
        let num_slices = ((1.0 / error_ratio).log2()).ceil() as u64;
        let slice_len_bits = (capacity as f64 / 2f64.ln()).ceil() as u64;
        let total_bits = num_slices * slice_len_bits;
        // round up to the next byte
        let buffer_bytes = total_bits.div_ceil(8) as usize;

        Bloom {
            buffer: vec![0; buffer_bytes].into_boxed_slice(),
            num_slices: NonZeroU64::new(num_slices).unwrap(),
        }
    }

    /// Create an index iterator for a given item.
    ///
    /// This creates an iterator of pairs `(byte, mask)` indices in the buffer.
    /// The iterator will return one pair of indexes for each slice in the bloom
    /// filter.
    ///
    /// The pairs `(byte idx, byte mask)` are:
    ///
    /// * byte idx: byte idx in `self.buffer` to be extract for usage with the
    ///   mask
    /// * byte mask: bit mask with a single bit set, can be ANDed (`&`) with
    ///   `self.buffer[idx]` to yield a number != 0 if the specified bit was
    ///   set. The mask can also be ORed (`|`) with the `self.buffer[idx]` to
    ///   set the corresponding bit.
    ///
    /// # Arguments
    ///
    /// * `h1` - The main hash
    /// * `h2` - The second hash (must be != 0)
    #[inline]
    fn index_iterator(
        &self,
        mut h1: u64,
        mut h2: u64,
    ) -> impl Iterator<Item = (usize, u8)> + use<> {
        // The _bit_ length (thus buffer.len() multiplied by 8) of each slice within
        // buffer. We'll use a NonZero type so that the compiler can avoid
        // checking for division/modulus by 0 inside the iterator.
        let slice_len = NonZeroU64::new(self.buffer.len() as u64 * 8 / self.num_slices).unwrap();

        // Generate `self.num_slices` hashes from 2 hashes, using enhanced double
        // hashing. See https://en.wikipedia.org/wiki/Double_hashing#Enhanced_double_hashing for details.
        // We choose to use 2x64 bit hashes instead of 2x32 ones as it gives
        // significant better false positive ratios.
        debug_assert_ne!(h2, 0, "Second hash can't be 0 for double hashing");
        (0..self.num_slices.get()).map(move |i| {
            // Calculate hash(i)
            let hi = h1 % slice_len + i * slice_len.get();
            // Advance enhanced double hashing state
            h1 = h1.wrapping_add(h2);
            h2 = h2.wrapping_add(i);
            // Resulting index/mask based on hash(i)
            let idx = (hi / 8) as usize;
            let mask = 1u8 << (hi % 8);
            (idx, mask)
        })
    }

    /// Insert an item identified by two hashes into the Bloom.
    ///
    /// # Arguments
    ///
    /// * `h1` - The main hash
    /// * `h2` - The second hash (must be != 0)
    #[inline]
    fn insert(&mut self, h1: u64, h2: u64) {
        // Set all bits (one per slice) corresponding to this item.
        //
        // Setting the bit:
        //    1000 0011 (self.buffer[idx])
        //    0001 0000 (mask)
        //    |---------
        //    1001 0011
        for (byte, mask) in self.index_iterator(h1, h2) {
            self.buffer[byte] |= mask;
        }
    }

    /// Test if item identified by two hashes is in the Bloom.
    ///
    /// # Arguments
    ///
    /// * `h1` - The main hash
    /// * `h2` - The second hash (must be != 0)
    #[inline]
    fn contains(&self, h1: u64, h2: u64) -> bool {
        // Check if all bits (one per slice) corresponding to this item are set.
        // See index_iterator comments for a detailed explanation.
        //
        // Potentially found case:
        //    0111 1111 (self.buffer[idx])
        //    0001 0000 (mask)
        //    &---------
        //    0001 0000 != 0
        //
        // Definitely not found case:
        //    1110 1111 (self.buffer[idx])
        //    0001 0000 (mask)
        //    &---------
        //    0000 0000 == 0
        self.index_iterator(h1, h2).all(|(byte, mask)| self.buffer[byte] & mask != 0)
    }
}

/// Return 2 hashes for `item` that can be used as h1 and h2 for double hashing.
///
/// See <https://en.wikipedia.org/wiki/Double_hashing#Enhanced_double_hashing>
/// for details.
#[inline]
fn double_hashing_hashes<T: Hash>(item: T) -> (u64, u64) {
    let mut hasher = StableHasher::new();
    item.hash(&mut hasher);
    let h1 = hasher.finish();

    // Write a nul byte to the existing state and get another hash.
    // This is appropriate when using a very high quality hasher,
    // which we know is the case.
    0u8.hash(&mut hasher);
    // h2 hash shouldn't be 0 for double hashing
    let h2 = hasher.finish().max(1);

    (h1, h2)
}

// From the paper:
// Considering the choice of s (GROWTH_FACTOR) = 2 for small expected growth and
// s = 4 for larger growth, one can see that r (TIGHTENING_RATIO) around 0.8
// - 0.9 is a sensible choice. Here we select good defaults for 10~1000x growth.
const DEFAULT_GROWTH_FACTOR: usize = 2;
const DEFAULT_TIGHTENING_RATIO: f64 = 0.8515625; // ~0.85 but has exact representation in f32/f64

const fn default_growth_factor() -> usize {
    DEFAULT_GROWTH_FACTOR
}

const fn default_tightening_ratio() -> f64 {
    DEFAULT_TIGHTENING_RATIO
}

/// A Growable Bloom Filter
///
/// # Overview
///
/// Implementation of [Scalable Bloom Filters] which also provides serde
/// serialization and deserialize.
///
/// A bloom filter lets you `insert` items, and then test association with
/// `contains`. It's space and time efficient, at the cost of false positives.
/// In particular, if `contains` returns `true`, it may be in filter.
/// But if `contains` returns false, it's definitely not in the bloom filter.
///
/// You can control the failure rate by setting `desired_error_prob` and
/// `est_insertions` appropriately.
///
/// # Applications
///
/// Bloom filters are typically used as a pre-cache to avoid expensive
/// operations. For example, if you need to ask ten thousand servers if they
/// have data XYZ, you could use GrowableBloom to figure out which ones do NOT
/// have XYZ.
///
/// # Example
///
/// ```rust
/// use client_common::bloom_filter::GrowableBloom;
///
/// // Create and insert into the bloom filter
/// let mut gbloom = GrowableBloom::new(0.05, 1000);
/// gbloom.insert(0);
/// assert!(gbloom.contains(0));
///
/// // Serialize and Deserialize the bloom filter
/// let s = serde_json::to_string(&gbloom).unwrap();
/// let des_gbloom: GrowableBloom = serde_json::from_str(&s).unwrap();
/// assert!(des_gbloom.contains(0));
///
/// // Builder API
/// use client_common::bloom_filter::GrowableBloomBuilder;
/// let mut gbloom = GrowableBloomBuilder::new()
///     .estimated_insertions(100)
///     .desired_error_ratio(0.05)
///     .build();
/// gbloom.insert(0);
/// assert!(gbloom.contains(0));
/// ```
///
/// [Scalable Bloom Filters]: https://gsd.di.uminho.pt/members/cbm/ps/dbloom.pdf
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct GrowableBloom {
    /// The constituent bloom filters
    #[serde(rename = "b")]
    blooms: Vec<Bloom>,
    #[serde(rename = "e")]
    desired_error_prob: f64,
    #[serde(rename = "t")]
    est_insertions: usize,
    /// Number of items successfully inserted
    #[serde(rename = "i")]
    inserts: usize,
    /// Item capacity
    #[serde(rename = "c")]
    capacity: usize,
    /// Growth factor
    #[serde(rename = "g", default = "default_growth_factor")]
    growth_factor: usize,
    #[serde(rename = "r", default = "default_tightening_ratio")]
    tightening_ratio: f64,
}

impl GrowableBloom {
    /// Create a new GrowableBloom filter.
    ///
    /// # Arguments
    ///
    /// * `desired_error_prob` - The desired error probability (eg. 0.05, 0.01)
    /// * `est_insertions` - The estimated number of insertions (eg. 100, 1000).
    ///
    /// Note: You really don't need to be accurate with est_insertions.
    ///       Power of 10 granularity should be fine (~1000 is decent).
    ///
    /// # Example
    ///
    /// ```rust
    /// // 5% failure rate, estimated 100 elements to insert
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut gbloom = GrowableBloom::new(0.05, 100);
    /// ```
    ///
    /// # Panics
    ///
    /// * Panics if desired_error_prob is less then 0 or greater than 1.
    /// * Panics if capacity is zero. If you're unsure, set it to 1000.
    ///
    /// # Builder API
    ///
    /// An alternative way to construct a GrowableBloom.
    ///
    /// See [`GrowableBloomBuilder`] for documentation. It allows you to specify
    /// other constants to control bloom filter behaviour.
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloomBuilder;
    /// let mut gbloom = GrowableBloomBuilder::new()
    ///     .estimated_insertions(100)
    ///     .desired_error_ratio(0.05)
    ///     .build();
    /// ```
    #[inline]
    pub fn new(desired_error_prob: f64, est_insertions: usize) -> GrowableBloom {
        Self::new_with_internals(
            desired_error_prob,
            est_insertions,
            DEFAULT_GROWTH_FACTOR,
            DEFAULT_TIGHTENING_RATIO,
        )
    }

    fn new_with_internals(
        desired_error_prob: f64,
        est_insertions: usize,
        growth_factor: usize,
        tightening_ratio: f64,
    ) -> GrowableBloom {
        assert!(0.0 < desired_error_prob && desired_error_prob < 1.0);
        assert!(growth_factor > 1);
        GrowableBloom {
            blooms: vec![],
            desired_error_prob,
            est_insertions,
            inserts: 0,
            capacity: 0,
            growth_factor,
            tightening_ratio,
        }
    }

    /// Test if `item` in the Bloom filter.
    ///
    /// If `true` is returned, it _may_ be in the filter.
    /// If `false` is returned, it's NOT in the filter.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to test
    ///
    /// # Example
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut bloom = GrowableBloom::new(0.05, 10);
    /// let item = 0;
    ///
    /// bloom.insert(item);
    /// assert!(bloom.contains(item));
    /// ```
    pub fn contains<T: Hash>(&self, item: T) -> bool {
        let (h1, h2) = double_hashing_hashes(item);
        self.blooms.iter().any(|bloom| bloom.contains(h1, h2))
    }

    /// Insert `item` into the filter.
    ///
    /// This may resize the GrowableBloom.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to insert
    ///
    /// # Example
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut bloom = GrowableBloom::new(0.05, 10);
    /// let item = 0;
    ///
    /// bloom.insert(item);
    /// bloom.insert(-1);
    /// bloom.insert(vec![1, 2, 3]);
    /// bloom.insert("hello");
    /// ```
    pub fn insert<T: Hash>(&mut self, item: T) -> bool {
        let (h1, h2) = double_hashing_hashes(item);
        // Step 1: Ask if we already have it
        if self.blooms.iter().any(|bloom| bloom.contains(h1, h2)) {
            return false;
        }
        // Step 2: Grow if necessary
        if self.inserts >= self.capacity {
            self.grow();
        }
        // Step 3: Insert it into the last
        self.inserts += 1;
        let curr_bloom = self.blooms.last_mut().unwrap();
        curr_bloom.insert(h1, h2);
        true
    }

    /// Clear the bloom filter.
    ///
    /// This does not resize the filter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut bloom = GrowableBloom::new(0.05, 10);
    /// let item = 0;
    ///
    /// bloom.insert(item);
    /// assert!(bloom.contains(item));
    /// bloom.clear();
    /// assert!(!bloom.contains(item)); // No longer contains item
    /// ```
    pub fn clear(&mut self) {
        self.blooms.clear();
        self.inserts = 0;
        self.capacity = 0;
    }

    /// Whether this bloom filter contain any items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inserts == 0
    }

    /// The current estimated number of elements added to the filter.
    /// This is an estimation, so it may or may not increase after
    /// an insertion in the filter.
    ///
    /// # Example
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut bloom = GrowableBloom::new(0.05, 10);
    ///
    /// bloom.insert(0);
    /// assert_eq!(bloom.len(), 1);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.inserts
    }

    /// The current estimated capacity of the filter.
    ///
    /// A filter starts with a small capacity and will expand to accommodate
    /// more items. The actual ratio of increase depends on the values used
    /// to construct the bloom filter.
    ///
    /// Note: An empty filter has capacity zero as we haven't calculated
    ///       the necessary bloom filter size. Subsequent inserts will result
    ///       in the capacity updating.
    ///
    /// # Example
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut bloom = GrowableBloom::new(0.05, 10);
    ///
    /// assert_eq!(bloom.capacity(), 0);
    ///
    /// bloom.insert(0);
    /// // After an insert, our capacity is no longer zero.
    /// assert_ne!(bloom.capacity(), 0);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Record if `item` already exists in the filter, and insert it if it
    /// doesn't already exist.
    ///
    /// Returns `true` if the item already existed in the filter.
    ///
    /// Note: This isn't faster than just inserting.
    ///
    /// # Example
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloom;
    /// let mut bloom = GrowableBloom::new(0.05, 10);
    /// let item = 0;
    ///
    /// let existed_before = bloom.check_and_set(item);
    /// assert!(existed_before == false);
    ///
    /// let existed_before = bloom.check_and_set(item);
    /// assert!(existed_before == true);
    /// ```
    pub fn check_and_set<T: Hash>(&mut self, item: T) -> bool {
        !self.insert(item)
    }

    /// Grow the GrowableBloom
    fn grow(&mut self) {
        // The paper gives an upper bound formula for the fp rate: fpUB <= fp0 * /
        // (1-r) This is because each sub bloom filter is created with an ever
        // smaller false-positive ratio, forming a geometric progression.
        // let r = TIGHTENING_RATIO
        // fpUB ~= fp0 * fp0*r * fp0*r*r * fp0*r*r*r ...
        // fp(x) = fp0 * (r**x)
        let error_ratio =
            self.desired_error_prob * self.tightening_ratio.powi(self.blooms.len() as _);
        // In order to have relatively small space overhead compared to a single
        // appropriately sized bloom filter the sub filters should be created
        // with increasingly bigger sizes.
        // let s = GROWTH_FACTOR
        // cap(x) = cap0 * (s**x)
        let capacity = self.est_insertions * self.growth_factor.pow(self.blooms.len() as _);
        let new_bloom = Bloom::new(capacity, error_ratio);
        self.blooms.push(new_bloom);
        self.capacity += capacity;
    }
}

/// Builder API for [`GrowableBloom`].
///
/// ```rust
/// use client_common::bloom_filter::GrowableBloomBuilder;
/// let mut gbloom = GrowableBloomBuilder::new()
///     .estimated_insertions(100)
///     .desired_error_ratio(0.05)
///     .build();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct GrowableBloomBuilder {
    desired_error_ratio: f64,
    est_insertions: usize,
    growth_factor: usize,
    tightening_ratio: f64,
}

impl Default for GrowableBloomBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GrowableBloomBuilder {
    /// Create a new GrowableBloomBuilder.
    ///
    /// Builder API for [`GrowableBloom`].
    ///
    /// ```rust
    /// use client_common::bloom_filter::GrowableBloomBuilder;
    /// let mut gbloom = GrowableBloomBuilder::new()
    ///     .estimated_insertions(1000)
    ///     .desired_error_ratio(0.01)
    ///     .growth_factor(2)
    ///     .tightening_ratio(0.85)
    ///     .build();
    /// gbloom.insert("hello world");
    /// assert!(gbloom.contains("hello world"));
    /// ```
    pub fn new() -> Self {
        Self {
            est_insertions: 1000,
            desired_error_ratio: 0.01,
            growth_factor: DEFAULT_GROWTH_FACTOR,
            tightening_ratio: DEFAULT_TIGHTENING_RATIO,
        }
    }

    /// Estimated number of insertions. A power of ten accuracy is good enough.
    ///
    /// # Panics
    ///
    /// This will panic in debug mode if count is zero.
    pub fn estimated_insertions(self, count: usize) -> Self {
        Self { est_insertions: count, ..self }
    }

    /// Desired error ratio (i.e. false positive rate).
    ///
    /// Smaller error ratios will use more memory and might be a bit slower.
    ///
    /// # Panics
    ///
    /// This will panic if the error ratio is outside of (0, 1.0).
    pub fn desired_error_ratio(self, ratio: f64) -> Self {
        Self { desired_error_ratio: ratio, ..self }
    }

    /// Base for the exponential growth factor.
    ///
    /// As more items are inserted into a [`GrowableBloom`] this growth_factor
    /// number is used to exponentially grow the capacity of newly added
    /// internal bloom filters. So this number is raised to some exponent
    /// proportional to the number of bloom filters held internally.
    ///
    /// Basically it'll control how quickly the bloom filter grows in capacity.
    /// By default it's set to two.
    pub fn growth_factor(self, factor: usize) -> Self {
        Self { growth_factor: factor, ..self }
    }

    /// Control the downwards adjustment on the error ratio when growing.
    ///
    /// When [`GrowableBloom`] adds a new internal bloom filter it uses
    /// the tightening_ratio to adjust the desired_error_ratio on these
    /// new, larger internal bloom filters. This is necessary to achieve decent
    /// accuracy on the user's desired error_ratio while using larger and larger
    /// bloom filters internally.
    ///
    /// By default this is set to ~0.85, but for smaller growth factors
    /// any number around 0.8 - 0.9 should be fine.
    ///
    /// # Panics
    ///
    /// This will panic if the ratio is outside of (0, 1.0).
    pub fn tightening_ratio(self, ratio: f64) -> Self {
        assert!(0.0 < ratio && ratio < 1.0);
        Self { tightening_ratio: ratio, ..self }
    }

    /// Consume the builder to create a [`GrowableBloom`].
    ///
    /// # Panics
    ///
    /// This will panic if an invalid value is specified.
    pub fn build(self) -> GrowableBloom {
        GrowableBloom::new_with_internals(
            self.desired_error_ratio,
            self.est_insertions,
            self.growth_factor,
            self.tightening_ratio,
        )
    }
}

#[cfg(test)]
mod tests {
    mod test_bloom {
        use crate::bloom_filter::{Bloom, double_hashing_hashes};

        #[test]
        fn can_insert_bloom() {
            let mut b = Bloom::new(100, 0.01);
            let (h1, h2) = double_hashing_hashes(123);
            b.insert(h1, h2);
            assert!(b.contains(h1, h2))
        }

        #[test]
        fn can_insert_string_bloom() {
            let mut b = Bloom::new(100, 0.01);
            let (h1, h2) = double_hashing_hashes("hello world".to_owned());
            b.insert(h1, h2);
            assert!(b.contains(h1, h2))
        }

        #[test]
        fn does_not_contain() {
            let mut b = Bloom::new(100, 0.01);
            let upper = 100;
            for i in (0..upper).step_by(2) {
                let (h1, h2) = double_hashing_hashes(i);
                b.insert(h1, h2);
                assert!(b.contains(h1, h2))
            }
            for i in (1..upper).step_by(2) {
                let (h1, h2) = double_hashing_hashes(i);
                assert!(!b.contains(h1, h2))
            }
        }

        #[test]
        fn can_insert_lots() {
            let mut b = Bloom::new(100, 0.01);
            for i in 0..1024 {
                let (h1, h2) = double_hashing_hashes(i);
                b.insert(h1, h2);
                assert!(b.contains(h1, h2))
            }
        }

        #[test]
        fn test_refs() {
            let item = String::from("Hello World");
            let mut b = Bloom::new(100, 0.01);
            let (h1, h2) = double_hashing_hashes(&item);
            b.insert(h1, h2);
            assert!(b.contains(h1, h2))
        }
    }

    mod test_growable {
        use crate::bloom_filter::{DEFAULT_TIGHTENING_RATIO, GrowableBloom};

        #[test]
        fn can_insert() {
            let mut b = GrowableBloom::new(0.05, 1000);
            let item = 20;
            b.insert(item);
            assert!(b.contains(item))
        }

        #[test]
        fn len_capacity_clear() {
            let mut b = GrowableBloom::new(0.05, 100);
            assert_eq!(b.len(), 0);
            assert_eq!(b.capacity(), 0);

            let item = 20;
            b.insert(item);
            assert_ne!(b.len(), 0);
            assert_ne!(b.capacity(), 0);

            b.clear();
            assert_eq!(b.len(), 0);
            assert_eq!(b.capacity(), 0);
        }

        #[test]
        fn ensure_capacity() {
            let mut b = GrowableBloom::new(0.05, 1);
            assert_eq!(b.capacity(), 0);
            b.insert("abc");
            assert_eq!(b.capacity(), 1);
            for i in 0..100 {
                b.insert(i);
            }
            assert_eq!(b.capacity(), 127);
        }

        #[test]
        fn can_insert_string() {
            let mut b = GrowableBloom::new(0.05, 1000);
            let item: String = "hello world".to_owned();
            b.insert(&item);
            assert!(b.contains(&item))
        }

        #[test]
        fn does_not_contain() {
            let mut b = GrowableBloom::new(0.05, 1000);
            assert!(!b.contains("hello"));
            b.insert(0);
            assert!(!b.contains("hello"));
            b.insert(1);
            assert!(!b.contains("hello"));
            b.insert(2);
            assert!(!b.contains("hello"));
        }

        #[test]
        fn can_insert_a_lot_of_elements() {
            let mut b = GrowableBloom::new(0.05, 1000);
            for i in 0..1000 {
                b.insert(i);
                assert!(b.contains(i));
            }
        }

        #[test]
        fn can_serialize_deserialize() {
            let mut b = GrowableBloom::new(0.05, 1000);
            b.insert(0);
            let s = serde_json::to_string(&b).unwrap();
            let b_s: GrowableBloom = serde_json::from_str(&s).unwrap();
            assert!(b_s.contains(0));
            assert!(!b_s.contains(1));
            assert!(!b_s.contains(1000));
        }

        /// A filter serialized by the `growable-bloom-filter` crate, before it
        /// was vendored into this crate, must still deserialize.
        #[test]
        fn can_deserialize_previously_persisted_filter() {
            // Serialized with `growable-bloom-filter` 2.1.1, after inserting the
            // numbers 0 to 9 into `GrowableBloom::new(0.05, 10)`.
            let previously_persisted = r#"{"b":[{"b":[20,119,205,134,19,51,155,133,113,23],"k":5}],"e":0.05,"t":10,"i":10,"c":10,"g":2,"r":0.8515625}"#;

            let b: GrowableBloom = serde_json::from_str(previously_persisted).unwrap();

            for i in 0..10 {
                assert!(b.contains(i), "{i} should be in the deserialized filter");
            }
            assert_eq!(b.len(), 10);
            assert_eq!(b.capacity(), 10);

            // And re-serializing it round-trips to the very same bytes.
            assert_eq!(serde_json::to_string(&b).unwrap(), previously_persisted);
        }

        /// Fields added after the original serialization format was defined
        /// must fall back to their defaults.
        #[test]
        fn can_deserialize_filter_without_growth_fields() {
            let without_growth_fields = r#"{"b":[{"b":[20,119,205,134,19,51,155,133,113,23],"k":5}],"e":0.05,"t":10,"i":10,"c":10}"#;

            let b: GrowableBloom = serde_json::from_str(without_growth_fields).unwrap();

            assert_eq!(b, serde_json::from_str::<GrowableBloom>(
                r#"{"b":[{"b":[20,119,205,134,19,51,155,133,113,23],"k":5}],"e":0.05,"t":10,"i":10,"c":10,"g":2,"r":0.8515625}"#
            ).unwrap());
        }

        #[test]
        fn verify_saturation() {
            for &fp in &[0.01, 0.001] {
                // The paper gives an upper bound formula for the fp rate: fpUB <= fp0*/(1-r)
                let fp_ub = fp / (1.0 - DEFAULT_TIGHTENING_RATIO);
                let initial_cap = 100u64;
                let growth = 1000u64;
                let mut b = GrowableBloom::new(fp, initial_cap as usize);
                // insert 1000x more elements than initially allocated
                for i in 1u64..=initial_cap * growth {
                    b.insert(i);

                    if i % (initial_cap * growth / 10) == 0
                        || [1, 2, 5, 10, 25].iter().any(|&g| i == initial_cap * g)
                    {
                        // A lot of tests are required to get a good estimate
                        let est_fp_rate = (i + 1..).take(50_000).filter(|i| b.contains(i)).count()
                            as f64
                            / 50_000.0;

                        assert!(est_fp_rate <= fp_ub);
                    }
                }
                for i in 1u64..=initial_cap * growth {
                    assert!(b.contains(i));
                }
            }
        }

        #[test]
        fn test_types_saturation() {
            let mut b = GrowableBloom::new(0.50, 100);
            b.insert(vec![1, 2, 3]);
            b.insert("hello");
            b.insert(-1);
            b.insert(0);
        }

        #[test]
        fn can_check_and_set() {
            let mut b = GrowableBloom::new(0.05, 1000);
            let item = 20;
            assert!(!b.check_and_set(item));
            assert!(b.check_and_set(item));
        }
    }

    mod test_builder {
        use crate::bloom_filter::GrowableBloomBuilder;

        #[test]
        fn can_build_bloom() {
            let mut gbloom = GrowableBloomBuilder::new().build();
            gbloom.insert(3);
            assert!(gbloom.contains(3));
        }

        #[test]
        #[should_panic]
        fn should_panic_on_bad_error_ratio() {
            GrowableBloomBuilder::new()
                .estimated_insertions(1000)
                .desired_error_ratio(99.9)
                .build();
        }

        #[test]
        #[should_panic]
        fn should_panic_on_too_small_tightening_ratio() {
            GrowableBloomBuilder::new().tightening_ratio(0.0).build();
        }

        #[test]
        #[should_panic]
        fn should_panic_on_too_large_tightening_ratio() {
            GrowableBloomBuilder::new().tightening_ratio(10.0).build();
        }

        #[test]
        fn can_specify_all_values() {
            // From https://github.com/dpbriggs/growable-bloom-filters/issues/7
            let mut gbloom = GrowableBloomBuilder::new()
                .estimated_insertions(3)
                .desired_error_ratio(0.00001)
                .tightening_ratio(0.5)
                .growth_factor(2)
                .build();
            for i in 0..100 {
                gbloom.insert(i);
            }
            for i in 0..100 {
                assert!(gbloom.contains(i));
            }
        }
    }
}
