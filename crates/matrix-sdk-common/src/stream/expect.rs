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
// Ported from tuwunel `src/core/utils/stream/expect.rs`, generalized over the
// error type rather than tuwunel's crate-wide `Error`.

//! Unwrapping adapter for fallible streams.

use std::fmt::Debug;

use futures_util::{Stream, StreamExt, TryStream};

/// Converts fallible stream items into values by expecting success.
///
/// Successful items pass through in source order. Errors terminate the current
/// poll by panicking with either a default or caller-supplied message.
pub trait TryExpect<Item, E>
where
    Item: Send,
    Self: Send + Sized,
{
    /// Expects every stream item with the default failure message.
    ///
    /// Successful values are unwrapped lazily as the stream is polled. The
    /// default message identifies a stream expectation failure.
    ///
    /// # Panics
    ///
    /// Panics when the source stream yields an error.
    fn expect_ok(self) -> impl Stream<Item = Item> + Send;

    /// Expects every stream item with `msg` as the failure message.
    ///
    /// Successful values are unwrapped lazily as the stream is polled. The
    /// supplied message is used for every failing item.
    ///
    /// # Panics
    ///
    /// Panics when the source stream yields an error.
    fn map_expect(self, msg: &str) -> impl Stream<Item = Item> + Send;
}

impl<Item, E, S> TryExpect<Item, E> for S
where
    S: Stream<Item = Result<Item, E>> + Send + TryStream,
    Item: Send,
    E: Debug,
    Self: Send + Sized,
{
    #[inline]
    fn expect_ok(self) -> impl Stream<Item = Item> + Send {
        self.map_expect("stream expectation failure")
    }

    #[inline]
    fn map_expect(self, msg: &str) -> impl Stream<Item = Item> + Send {
        self.map(|res| res.expect(msg))
    }
}
