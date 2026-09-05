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

//! Caches that let a synchronous algorithm read from an asynchronous store.
//!
//! Each cache holds `Option<E>` per key: a present value, or a recorded
//! absence. Absences matter, because a state key the room genuinely lacks (no
//! power levels event, say) must not look like a key that has yet to be
//! fetched, or the caller would fetch it on every round forever.
//!
//! A lookup for a key the cache has never seen records a miss. The caller
//! resolves the misses between rounds, which is what turns an async store into
//! something the synchronous algorithms can read.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
};

/// How many times a check re-runs to resolve newly discovered lookups.
///
/// Each round resolves every miss recorded by the previous one, and the seeds
/// cover what the specification says a check reads, so a round limit this size
/// is only reached by an algorithm reading something entirely unanticipated.
pub const MAX_FETCH_ROUNDS: usize = 16;

/// A key-value cache recording lookups it could not answer.
#[derive(Debug)]
pub(crate) struct FetchCache<K, V> {
    entries: HashMap<K, Option<V>>,
    misses: RefCell<HashSet<K>>,
}

impl<K, V> FetchCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Creates an empty cache.
    pub(crate) fn new() -> Self {
        Self { entries: HashMap::new(), misses: RefCell::new(HashSet::new()) }
    }

    /// Reads a key, recording a miss when the cache has never seen it.
    ///
    /// A recorded absence answers `None` without recording a miss, so an entry
    /// that was fetched and found missing is not fetched again.
    pub(crate) fn get(&self, key: &K) -> Option<V> {
        match self.entries.get(key) {
            Some(entry) => entry.clone(),
            None => {
                self.misses.borrow_mut().insert(key.clone());
                None
            }
        }
    }

    /// Records the outcome of fetching a key, present or absent.
    pub(crate) fn insert(&mut self, key: K, value: Option<V>) {
        self.entries.insert(key, value);
    }

    /// Whether the cache already holds an outcome for a key.
    pub(crate) fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Removes and returns the misses recorded since the last call.
    pub(crate) fn take_misses(&mut self) -> HashSet<K> {
        self.misses.take()
    }
}
