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
// Ported from tuwunel `src/core/utils/stream/try_tools.rs`.

//! General-purpose operations for `futures_util::TryStream`.

#![allow(clippy::type_complexity)]

use futures_util::{TryStream, TryStreamExt, future, future::Ready, stream::TryTakeWhile};

/// Adds general-purpose operations to fallible streams.
///
/// The adapters preserve the source error type and successful item order. They
/// operate lazily without collecting the stream.
pub trait TryTools<T, E, S>
where
    S: TryStream<Ok = T, Error = E, Item = Result<T, E>> + ?Sized,
    Self: TryStream + Sized,
{
    /// Limits the stream to at most `n` successful items.
    ///
    /// After yielding `n` successes, the adapter consumes one additional
    /// success to detect the limit. Earlier source errors are still forwarded;
    /// with zero, they precede consumption of the first unyielded success.
    fn try_take(
        self,
        n: usize,
    ) -> TryTakeWhile<
        Self,
        Ready<Result<bool, S::Error>>,
        impl FnMut(&S::Ok) -> Ready<Result<bool, S::Error>>,
    >;
}

impl<T, E, S> TryTools<T, E, S> for S
where
    S: TryStream<Ok = T, Error = E, Item = Result<T, E>> + ?Sized,
    Self: TryStream + Sized,
{
    #[inline]
    fn try_take(
        self,
        mut n: usize,
    ) -> TryTakeWhile<
        Self,
        Ready<Result<bool, S::Error>>,
        impl FnMut(&S::Ok) -> Ready<Result<bool, S::Error>>,
    > {
        self.try_take_while(move |_| {
            let res = future::ok(n > 0);
            n = n.saturating_sub(1);
            res
        })
    }
}
