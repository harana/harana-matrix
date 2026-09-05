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
// Ported from tuwunel `src/core/utils/stream/ignore.rs`, with two deviations:
// the error type is generic rather than tuwunel's crate-wide `Error`, and
// `ignore_err` logs and filters in every build. Tuwunel panics on a dropped
// error in debug builds, which is not appropriate for a library whose store and
// network errors are expected to occur in a consumer's debug build too.

//! Selection of successful or failed items from a result stream.

use std::fmt::Debug;

use futures_util::{Stream, StreamExt, future::ready};
use tracing::warn;

/// Selects successful or failed items from a result stream.
pub trait TryIgnore<Item, E>
where
    Item: Send,
    Self: Send + Sized,
{
    /// Yields successful values, logging and discarding errors.
    ///
    /// Each error is reported at `warn` level before being dropped, so a
    /// silently truncated stream remains diagnosable.
    fn ignore_err(self) -> impl Stream<Item = Item> + Send;

    /// Yields errors while ignoring successful values.
    ///
    /// Successful items are filtered out as the stream is polled. Errors retain
    /// their source order and value.
    fn ignore_ok(self) -> impl Stream<Item = E> + Send;
}

impl<Item, E, S> TryIgnore<Item, E> for S
where
    S: Stream<Item = Result<Item, E>> + Send,
    Item: Send,
    E: Debug + Send,
    Self: Send + Sized,
{
    #[inline]
    fn ignore_err(self) -> impl Stream<Item = Item> + Send {
        self.filter_map(|res| {
            ready(match res {
                Ok(item) => Some(item),
                Err(error) => {
                    warn!(?error, "ignoring stream error");
                    None
                }
            })
        })
    }

    #[inline]
    fn ignore_ok(self) -> impl Stream<Item = E> + Send {
        self.filter_map(|res| ready(res.err()))
    }
}
