// Copyright 2026 The Matrix.org Foundation C.I.C.
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

//! The [Tokio] backend of the [`AsyncRuntime`] abstraction.
//!
//! [Tokio]: https://tokio.rs

use std::time::Duration;

use super::AsyncRuntime;
use crate::common::BoxFuture;

/// An [`AsyncRuntime`] backed by Tokio.
///
/// This is the runtime the SDK uses on non-Wasm targets unless another one was
/// installed with [`set_runtime`](super::set_runtime). Every method requires
/// the caller to be inside a Tokio runtime context.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct TokioRuntime;

impl AsyncRuntime for TokioRuntime {
    fn spawn(&self, future: BoxFuture<'static, ()>) {
        tokio::spawn(future);
    }

    fn spawn_blocking(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        tokio::task::spawn_blocking(task);
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        Box::pin(tokio::time::sleep(duration))
    }
}
