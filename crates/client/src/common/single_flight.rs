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

//! Coalescing of concurrent work onto one execution per key.
//!
//! The dedup idea is tuwunel's (`src/service/fetcher`), where a single worker
//! owns every in-flight federation fetch and later callers subscribe to the one
//! already running. This is that behavior without the worker: callers contend
//! for the key, the first runs the work, and the rest read what it produced.
//!
//! Consequently there is nothing to cancel. Tuwunel's fetch is owned by a task
//! and needs a liveness token so it stops when its callers lose interest; here
//! the work runs inside a caller's own future, so dropping that caller drops
//! the work, and the next contender starts it afresh.
//!
//! Nothing is cached: an entry lives exactly as long as some caller holds or
//! awaits it, so a call that starts after the last one finished does the work
//! again.

use std::{fmt::Debug, hash::Hash};

use crate::common::mutex_map::MutexMap;

/// Runs one execution of some work per key, however many callers ask for it.
///
/// Callers that arrive while the work is running wait for it and receive a
/// clone of its result. This is the deduplication half of a request cache,
/// without the caching: two concurrent media downloads of one URI make one
/// request, but a later download makes another.
#[derive(Debug)]
pub struct SingleFlight<Key, Val> {
    inflight: MutexMap<Key, Option<Val>>,
}

impl<Key, Val> SingleFlight<Key, Val>
where
    Key: Clone + Eq + Hash + Send,
    Val: Clone + Send,
{
    /// Creates an empty set of in-flight work.
    #[must_use]
    pub fn new() -> Self {
        Self { inflight: MutexMap::new() }
    }

    /// Runs `f` for `key`, or waits for the run already in progress.
    ///
    /// Exactly one caller runs `f` while the others wait, and every caller
    /// receives the same value. If the caller running `f` is dropped before it
    /// completes, the next waiting caller runs `f` itself rather than waiting
    /// for a result that will never arrive.
    ///
    /// `f` is a closure rather than a future so that the waiting callers never
    /// construct one they will not run.
    pub async fn run<K, F, Fut>(&self, key: &K, f: F) -> Val
    where
        K: Debug + Send + ?Sized + Sync + ToOwned<Owned = Key>,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Val>,
    {
        let mut guard = self.inflight.lock(key).await;

        // Set by a caller that ran while this one was waiting for the key.
        if let Some(value) = guard.as_ref() {
            return value.clone();
        }

        let value = f().await;
        *guard = Some(value.clone());

        value
    }

    /// The number of keys currently held or awaited.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inflight.len()
    }

    /// Whether no key is currently held or awaited.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inflight.is_empty()
    }
}

impl<Key, Val> Default for SingleFlight<Key, Val>
where
    Key: Clone + Eq + Hash + Send,
    Val: Clone + Send,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use futures_channel::oneshot;
    use futures_util::{
        FutureExt,
        future::{join, join_all, pending},
    };
    use harana_matrix_macros::async_test;

    use super::SingleFlight;

    #[async_test]
    async fn test_concurrent_callers_share_one_run() {
        let flight: SingleFlight<String, usize> = SingleFlight::new();
        let runs = Arc::new(AtomicUsize::new(0));

        // The work parks on this gate, so the three calls genuinely overlap
        // rather than each completing before the next is polled.
        let (open, gate) = oneshot::channel::<()>();
        let gate = gate.shared();

        let call = || {
            let runs = runs.clone();
            let gate = gate.clone();
            flight.run("key", move || async move {
                let _ = gate.await;
                runs.fetch_add(1, Ordering::SeqCst)
            })
        };

        let mut calls = Box::pin(join_all([call(), call(), call()]));
        assert!((&mut calls).now_or_never().is_none());

        open.send(()).unwrap();
        let results = calls.await;

        assert_eq!(runs.load(Ordering::SeqCst), 1);
        assert_eq!(results, [0, 0, 0]);
    }

    #[async_test]
    async fn test_a_later_call_runs_again() {
        let flight: SingleFlight<String, usize> = SingleFlight::new();
        let runs = Arc::new(AtomicUsize::new(0));

        let call = || {
            let runs = runs.clone();
            flight.run("key", move || async move { runs.fetch_add(1, Ordering::SeqCst) })
        };

        assert_eq!(call().await, 0);
        // Nothing is cached, so the second call does the work again.
        assert_eq!(call().await, 1);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        assert!(flight.is_empty());
    }

    #[async_test]
    async fn test_distinct_keys_do_not_coalesce() {
        let flight: SingleFlight<String, &str> = SingleFlight::new();

        let (one, two) =
            join(flight.run("one", || async { "one" }), flight.run("two", || async { "two" }))
                .await;

        assert_eq!((one, two), ("one", "two"));
    }

    #[async_test]
    async fn test_a_dropped_runner_hands_the_work_to_the_next_caller() {
        let flight: SingleFlight<String, usize> = SingleFlight::new();
        let runs = Arc::new(AtomicUsize::new(0));

        // The first caller takes the key and never completes.
        let abandoned = {
            let runs = runs.clone();
            Box::pin(flight.run("key", move || async move {
                runs.fetch_add(1, Ordering::SeqCst);
                pending::<usize>().await
            }))
        };

        // `now_or_never` polls once, which is enough for the caller to take the
        // key and park on the work that never finishes.
        let mut abandoned = abandoned;
        assert!((&mut abandoned).now_or_never().is_none());

        let mut waiting = {
            let runs = runs.clone();
            Box::pin(flight.run("key", move || async move { runs.fetch_add(1, Ordering::SeqCst) }))
        };

        // Blocked: the abandoned caller holds the key.
        assert!((&mut waiting).now_or_never().is_none());
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        // Releasing the key hands the work over rather than stranding it.
        drop(abandoned);
        assert_eq!(waiting.await, 1);
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }
}
