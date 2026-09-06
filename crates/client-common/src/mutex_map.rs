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
// Ported from tuwunel `src/core/utils/mutex_map.rs`, with two deviations: the
// keyed mutex is `futures_util::lock::Mutex` rather than Tokio's, so the type
// works under any runtime and on Wasm, and the fallible acquisitions return
// `Option` rather than tuwunel's crate-wide error type.

//! Per-key asynchronous mutual exclusion with automatic entry cleanup.
//!
//! Each key maps to a mutex shared by its current contenders. The last
//! contender to release its claim removes the entry, whether it held the mutex
//! or was canceled while waiting for it.

use std::{fmt::Debug, hash::Hash, sync::Arc};

use futures_util::lock::OwnedMutexGuard;

use crate::locks::Mutex as StdMutex;

/// Provides independent asynchronous mutexes keyed by owned values.
///
/// Lock acquisition creates entries on demand, and callers contending for the
/// same key serialize. An entry lives exactly as long as some caller holds or
/// contends for it, so an idle map holds no per-key state.
#[derive(Debug)]
pub struct MutexMap<Key, Val> {
    map: Map<Key, Val>,
}

/// Keeps a keyed mutex locked until the guard is dropped.
///
/// The guard retains the parent map so cleanup remains possible. Dropping it
/// releases the keyed mutex and then removes the entry when no other holder or
/// contender references it.
#[derive(Debug)]
#[clippy::has_significant_drop]
pub struct MutexMapGuard<Key, Val> {
    map: Map<Key, Val>,
    entry: Option<Value<Val>>,
    val: Option<OwnedMutexGuard<Val>>,
}

type Map<Key, Val> = Arc<MapMutex<Key, Val>>;
type MapMutex<Key, Val> = StdMutex<HashMap<Key, Val>>;
type HashMap<Key, Val> = std::collections::HashMap<Key, Value<Val>>;
type Value<Val> = Arc<futures_util::lock::Mutex<Val>>;

impl<Key, Val> MutexMap<Key, Val>
where
    Key: Clone + Eq + Hash + Send,
    Val: Default + Send,
{
    /// Creates an empty keyed mutex map.
    ///
    /// No per-key mutex is allocated until a lock method first sees its key.
    /// The result is equivalent to [`Default::default`].
    #[must_use]
    pub fn new() -> Self {
        Self { map: Map::new(MapMutex::new(HashMap::new())) }
    }

    /// Acquires the asynchronous mutex associated with a key.
    ///
    /// The method creates an entry if absent and waits for the current holder
    /// to release it. Cancellation while waiting releases the claim on the
    /// entry.
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn lock<K>(&self, k: &K) -> MutexMapGuard<Key, Val>
    where
        K: Debug + Send + ?Sized + Sync + ToOwned<Owned = Key>,
    {
        self.entry(k).lock().await
    }

    /// Attempts to acquire a key without waiting for its asynchronous mutex.
    ///
    /// The key entry is created if absent, and contention returns `None`
    /// instead of yielding.
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn try_lock<K>(&self, k: &K) -> Option<MutexMapGuard<Key, Val>>
    where
        K: Debug + Send + ?Sized + Sync + ToOwned<Owned = Key>,
    {
        self.entry(k).try_lock()
    }

    /// Reports whether the map currently contains an entry for a key.
    ///
    /// An entry represents a held mutex or contenders that still reference it.
    #[must_use]
    pub fn contains(&self, k: &Key) -> bool {
        self.map.lock().contains_key(k)
    }

    /// Reports whether no keyed mutex entries are currently tracked.
    ///
    /// A false result implies at least one active holder or contender.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.lock().is_empty()
    }

    /// Returns the number of keyed mutex entries currently tracked.
    ///
    /// The count includes held mutexes and entries retained by contenders.
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.lock().len()
    }

    fn entry<K>(&self, k: &K) -> MutexMapGuard<Key, Val>
    where
        K: ?Sized + ToOwned<Owned = Key>,
    {
        let val = self.map.lock().entry(k.to_owned()).or_default().clone();

        MutexMapGuard { map: Arc::clone(&self.map), entry: Some(val), val: None }
    }
}

impl<Key, Val> Default for MutexMap<Key, Val>
where
    Key: Clone + Eq + Hash + Send,
    Val: Default + Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Key, Val> std::ops::Deref for MutexMapGuard<Key, Val> {
    type Target = Val;

    fn deref(&self) -> &Val {
        self.val.as_ref().expect("locked")
    }
}

impl<Key, Val> std::ops::DerefMut for MutexMapGuard<Key, Val> {
    fn deref_mut(&mut self) -> &mut Val {
        self.val.as_mut().expect("locked")
    }
}

impl<Key, Val> MutexMapGuard<Key, Val> {
    async fn lock(mut self) -> Self {
        // The in-flight claim must release before this guard, so a cancellation
        // leaves the entry unreferenced.
        let val = self.claim();

        self.val = Some(val.lock_owned().await);
        self
    }

    fn try_lock(mut self) -> Option<Self> {
        self.val = Some(self.claim().try_lock_owned()?);

        Some(self)
    }

    fn claim(&self) -> Value<Val> {
        Arc::clone(self.entry.as_ref().expect("claimed"))
    }
}

impl<Key, Val> Drop for MutexMapGuard<Key, Val> {
    #[tracing::instrument(name = "unlock", level = "trace", skip_all)]
    fn drop(&mut self) {
        self.val.take();

        // Releasing the claim under the map lock elects the last one out.
        let mut map = self.map.lock();

        if self.entry.take().is_some_and(|val| Arc::strong_count(&val) <= 2) {
            map.retain(|_, val| Arc::strong_count(val) > 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use common_test_macros::async_test;

    use super::MutexMap;

    #[async_test]
    async fn test_entries_are_reaped_when_the_last_holder_drops() {
        let map: MutexMap<String, ()> = MutexMap::new();
        assert!(map.is_empty());

        {
            let _guard = map.lock("a").await;
            assert_eq!(map.len(), 1);
            assert!(map.contains(&"a".to_owned()));

            let _other = map.lock("b").await;
            assert_eq!(map.len(), 2);
        }

        assert!(map.is_empty());
    }

    #[async_test]
    async fn test_a_held_key_cannot_be_acquired_again() {
        let map: MutexMap<String, ()> = MutexMap::new();

        let guard = map.lock("a").await;
        assert!(map.try_lock("a").is_none());
        assert!(map.try_lock("b").is_some());

        drop(guard);
        assert!(map.try_lock("a").is_some());
    }
}
