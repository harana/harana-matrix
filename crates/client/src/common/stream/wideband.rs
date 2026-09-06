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
// Ported from tuwunel `src/core/utils/stream/wideband.rs`.

//! Wideband stream combinator extensions to `futures_util::Stream`.

use std::{convert::identity, future::Future};

use futures_util::stream::{Stream, StreamExt};

use super::{ReadyExt, automatic_width};

/// Adds bounded concurrent transformations that preserve stream order.
///
/// Multiple item futures may run ahead of downstream demand. Completed outputs
/// are held until every earlier input has produced its output. Use
/// [`super::BroadbandExt`] when completion order is acceptable.
pub trait WidebandExt<Item>
where
    Self: Stream<Item = Item> + Send + Sized,
{
    /// Maps and filters items concurrently with an explicit width.
    ///
    /// `n` limits in-flight item futures, while `None` selects the automatic
    /// width; an explicit zero cannot make progress. Present outputs retain
    /// source order and absent outputs are omitted.
    fn widen_filter_map<F, Fut, U, N>(self, n: N, f: F) -> impl Stream<Item = U> + Send
    where
        N: Into<Option<usize>>,
        F: Fn(Item) -> Fut + Send,
        Fut: Future<Output = Option<U>> + Send,
        U: Send;

    /// Maps items concurrently with an explicit width while preserving order.
    ///
    /// `n` limits in-flight item futures, while `None` selects the automatic
    /// width; an explicit zero cannot make progress. Item futures may run
    /// ahead, but outputs retain source order.
    fn widen_then<F, Fut, U, N>(self, n: N, f: F) -> impl Stream<Item = U> + Send
    where
        N: Into<Option<usize>>,
        F: Fn(Item) -> Fut + Send,
        Fut: Future<Output = U> + Send,
        U: Send;

    /// Maps and filters items concurrently with the automatic width.
    ///
    /// Present outputs retain source order even when their futures complete out
    /// of order. Absent outputs are omitted.
    #[inline]
    fn wide_filter_map<F, Fut, U>(self, f: F) -> impl Stream<Item = U> + Send
    where
        F: Fn(Item) -> Fut + Send,
        Fut: Future<Output = Option<U>> + Send,
        U: Send,
    {
        self.widen_filter_map(None, f)
    }

    /// Maps items concurrently with the automatic width while preserving order.
    ///
    /// Item futures may run ahead of downstream demand. Completed outputs wait
    /// for every earlier input before being yielded.
    #[inline]
    fn wide_then<F, Fut, U>(self, f: F) -> impl Stream<Item = U> + Send
    where
        F: Fn(Item) -> Fut + Send,
        Fut: Future<Output = U> + Send,
        U: Send,
    {
        self.widen_then(None, f)
    }
}

impl<Item, S> WidebandExt<Item> for S
where
    S: Stream<Item = Item> + Send + Sized,
{
    #[inline]
    fn widen_filter_map<F, Fut, U, N>(self, n: N, f: F) -> impl Stream<Item = U> + Send
    where
        N: Into<Option<usize>>,
        F: Fn(Item) -> Fut + Send,
        Fut: Future<Output = Option<U>> + Send,
        U: Send,
    {
        self.map(f).buffered(n.into().unwrap_or_else(automatic_width)).ready_filter_map(identity)
    }

    #[inline]
    fn widen_then<F, Fut, U, N>(self, n: N, f: F) -> impl Stream<Item = U> + Send
    where
        N: Into<Option<usize>>,
        F: Fn(Item) -> Fut + Send,
        Fut: Future<Output = U> + Send,
        U: Send,
    {
        self.map(f).buffered(n.into().unwrap_or_else(automatic_width))
    }
}
