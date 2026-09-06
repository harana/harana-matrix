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
// Ported from tuwunel `src/core/utils/stream/cloned.rs`.

//! Cloning adapter for streams of shared references.

use futures_util::{Stream, StreamExt, stream::Map};

/// Clones items yielded by a stream of shared references.
///
/// Each referenced value is cloned only when its stream item is polled. Item
/// order and readiness follow the source stream unchanged.
pub trait Cloned<'a, T, S>
where
    S: Stream<Item = &'a T>,
    T: Clone + 'a,
{
    /// Returns a stream of owned clones from the borrowed items.
    ///
    /// The adapter uses [`Clone::clone`] for each yielded reference. It
    /// performs no eager collection or buffering.
    fn cloned(self) -> Map<S, fn(&T) -> T>;
}

impl<'a, T, S> Cloned<'a, T, S> for S
where
    S: Stream<Item = &'a T>,
    T: Clone + 'a,
{
    #[inline]
    fn cloned(self) -> Map<S, fn(&T) -> T> {
        self.map(Clone::clone)
    }
}
