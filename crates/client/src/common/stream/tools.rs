// Copyright 2025 Tuwunel Contributors
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
// Ported from tuwunel `src/core/utils/stream/tools.rs`. Tuwunel's reservoir
// sampler (`sample_by`) is omitted here because it would pull `arrayvec` and
// `rand` into this crate for no current consumer.

//! Aggregation and folding operations for streams.

use std::{collections::HashMap, future::Future, hash::Hash};

use futures_util::{Stream, StreamExt};

use super::ReadyExt;

/// Adds aggregation and folding operations to streams.
///
/// Counting adapters consume the source into hash maps, while folding avoids an
/// intermediate collection.
pub trait Tools<Item>
where
    Self: Stream<Item = Item> + Send + Sized,
    <Self as Stream>::Item: Send,
{
    /// Counts occurrences of each distinct stream item.
    ///
    /// The entire stream is consumed into a hash map that starts at zero
    /// capacity and grows as needed. Equal items share one incrementing
    /// counter.
    ///
    /// # Panics
    ///
    /// Panics if an item's occurrence count overflows `usize`.
    fn counts(self) -> impl Future<Output = HashMap<Item, usize>> + Send
    where
        <Self as Stream>::Item: Eq + Hash;

    /// Counts occurrences of keys derived from stream items.
    ///
    /// `f` is applied once to every item before counting. The result map starts
    /// at zero capacity and grows as needed.
    ///
    /// # Panics
    ///
    /// Panics if a derived key's occurrence count overflows `usize`.
    fn counts_by<K, F>(self, f: F) -> impl Future<Output = HashMap<K, usize>> + Send
    where
        F: Fn(Item) -> K + Send,
        K: Eq + Hash + Send;

    /// Counts derived keys into a map with initial capacity `CAP`.
    ///
    /// `f` is applied once to every stream item. The map initially reserves
    /// space for at least `CAP` distinct keys and grows as needed.
    ///
    /// # Panics
    ///
    /// Panics if a derived key's occurrence count overflows `usize`.
    fn counts_by_with_cap<const CAP: usize, K, F>(
        self,
        f: F,
    ) -> impl Future<Output = HashMap<K, usize>> + Send
    where
        F: Fn(Item) -> K + Send,
        K: Eq + Hash + Send;

    /// Counts items into a map with initial capacity `CAP`.
    ///
    /// Equal items share one counter as the entire stream is consumed. The map
    /// initially reserves space for at least `CAP` distinct items and grows as
    /// needed.
    ///
    /// # Panics
    ///
    /// Panics if an item's occurrence count overflows `usize`.
    fn counts_with_cap<const CAP: usize>(self) -> impl Future<Output = HashMap<Item, usize>> + Send
    where
        <Self as Stream>::Item: Eq + Hash;

    /// Folds the stream from `T::default()` with an asynchronous accumulator.
    ///
    /// Each item is processed in source order after the previous fold future
    /// resolves. The final accumulator is returned when the stream ends.
    fn fold_default<T, F, Fut>(self, f: F) -> impl Future<Output = T> + Send
    where
        F: Fn(T, Item) -> Fut + Send,
        Fut: Future<Output = T> + Send,
        T: Default + Send;
}

impl<Item, S> Tools<Item> for S
where
    S: Stream<Item = Item> + Send + Sized,
    <Self as Stream>::Item: Send,
{
    #[inline]
    fn counts(self) -> impl Future<Output = HashMap<Item, usize>> + Send
    where
        <Self as Stream>::Item: Eq + Hash,
    {
        self.counts_with_cap::<0>()
    }

    #[inline]
    fn counts_by<K, F>(self, f: F) -> impl Future<Output = HashMap<K, usize>> + Send
    where
        F: Fn(Item) -> K + Send,
        K: Eq + Hash + Send,
    {
        self.counts_by_with_cap::<0, K, F>(f)
    }

    #[inline]
    fn counts_by_with_cap<const CAP: usize, K, F>(
        self,
        f: F,
    ) -> impl Future<Output = HashMap<K, usize>> + Send
    where
        F: Fn(Item) -> K + Send,
        K: Eq + Hash + Send,
    {
        self.map(f).counts_with_cap::<CAP>()
    }

    #[inline]
    fn counts_with_cap<const CAP: usize>(self) -> impl Future<Output = HashMap<Item, usize>> + Send
    where
        <Self as Stream>::Item: Eq + Hash,
    {
        self.ready_fold(HashMap::with_capacity(CAP), |mut counts, item| {
            let entry: &mut usize = counts.entry(item).or_default();
            *entry = entry.checked_add(1).expect("item count overflowed");
            counts
        })
    }

    #[inline]
    fn fold_default<T, F, Fut>(self, f: F) -> impl Future<Output = T> + Send
    where
        F: Fn(T, Item) -> Fut + Send,
        Fut: Future<Output = T> + Send,
        T: Default + Send,
    {
        self.fold(T::default(), f)
    }
}
