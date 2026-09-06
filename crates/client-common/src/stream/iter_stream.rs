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
// Ported from tuwunel `src/core/utils/stream/iter_stream.rs`, generalized over
// the error type rather than tuwunel's crate-wide `Error`.

//! Conversion of synchronous iterables into ready streams.

use futures_util::{
    StreamExt, stream,
    stream::{Stream, TryStream},
};

/// Converts synchronous iterables into immediately ready streams.
///
/// Source iteration order is preserved. The fallible form wraps each item in a
/// successful result whose error type is chosen by the caller.
pub trait IterStream<I: IntoIterator + Send> {
    /// Converts the iterable into a stream of its items.
    ///
    /// Items are yielded in the source iterator's order. Polling requires no
    /// asynchronous work beyond advancing that iterator.
    fn stream(self) -> impl Stream<Item = <I as IntoIterator>::Item> + Send;

    /// Converts the iterable into a stream of successful results.
    ///
    /// Every source item is wrapped in `Ok`. The adapter itself never produces
    /// an error, so `E` is only ever inferred from the consumer.
    fn try_stream<E>(
        self,
    ) -> impl TryStream<
        Ok = <I as IntoIterator>::Item,
        Error = E,
        Item = Result<<I as IntoIterator>::Item, E>,
    > + Send;
}

impl<I> IterStream<I> for I
where
    I: IntoIterator + Send,
    <I as IntoIterator>::IntoIter: Send,
{
    #[inline]
    fn stream(self) -> impl Stream<Item = <I as IntoIterator>::Item> + Send {
        stream::iter(self)
    }

    #[inline]
    fn try_stream<E>(
        self,
    ) -> impl TryStream<
        Ok = <I as IntoIterator>::Item,
        Error = E,
        Item = Result<<I as IntoIterator>::Item, E>,
    > + Send {
        self.stream().map(Ok)
    }
}
