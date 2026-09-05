// Copyright 2025 The Matrix.org Foundation C.I.C.
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

//! Platform-specific stream utilities and stream combinator extensions.
//!
//! This module provides a unified `BoxStream` + `StreamExt` class for working
//! with boxed streams across different platforms. On native platforms,
//! streams can be `Send`, but on Wasm they cannot. This module abstracts
//! over that difference.
//!
//! It also extends [`futures_util::Stream`] and [`futures_util::TryStream`]
//! with ordered and completion-ordered concurrency ([`WidebandExt`] and
//! [`BroadbandExt`]), synchronous closure variants of the async combinators
//! ([`ReadyExt`], [`TryReadyExt`]), and result handling. The concurrency
//! adapters are ported from tuwunel (`src/core/utils/stream`), which is
//! likewise Apache-2.0 licensed.

#[cfg(not(target_family = "wasm"))]
mod sys {
    // On native platforms, just re-export everything from futures_util
    pub use futures_util::{StreamExt, stream::BoxStream};
}

#[cfg(target_family = "wasm")]
mod sys {
    use futures_core::Stream;
    // On Wasm, BoxStream is LocalBoxStream
    pub use futures_util::stream::LocalBoxStream as BoxStream;

    /// Custom `StreamExt` trait for Wasm that provides essential methods
    /// like `.boxed()` and `.next()` without `Send` requirements.
    pub trait StreamExt: Stream {
        /// Box this stream using `LocalBoxStream` (no `Send` requirement).
        fn boxed<'a>(self) -> BoxStream<'a, Self::Item>
        where
            Self: Sized + 'a,
        {
            futures_util::StreamExt::boxed_local(self)
        }

        /// Get the next item from this stream.
        fn next(&mut self) -> futures_util::stream::Next<'_, Self>
        where
            Self: Unpin,
        {
            futures_util::StreamExt::next(self)
        }
    }

    impl<S: Stream> StreamExt for S {}
}

mod band;
mod broadband;
mod cloned;
mod expect;
mod ignore;
mod iter_stream;
mod ready;
mod tools;
mod try_broadband;
mod try_ready;
mod try_tools;
mod try_wideband;
mod wideband;

pub use sys::*;

pub use self::{
    band::{
        AMPLIFICATION_LIMIT, WIDTH_LIMIT, automatic_amplification, automatic_width,
        set_amplification, set_width,
    },
    broadband::BroadbandExt,
    cloned::Cloned,
    expect::TryExpect,
    ignore::TryIgnore,
    iter_stream::IterStream,
    ready::ReadyExt,
    tools::Tools,
    try_broadband::TryBroadbandExt,
    try_ready::TryReadyExt,
    try_tools::TryTools,
    try_wideband::TryWidebandExt,
    wideband::WidebandExt,
};
