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

//! The Wasm backend of the [`AsyncRuntime`] abstraction, spawning onto the JS
//! event loop.

use std::time::Duration;

use super::AsyncRuntime;
use crate::common::BoxFuture;

/// An [`AsyncRuntime`] running on the JavaScript event loop.
///
/// Tasks are spawned with `wasm-bindgen-futures` and timers use
/// `setTimeout` through `gloo-timers`. This is the runtime the SDK uses on Wasm
/// targets unless another one was installed with
/// [`set_runtime`](super::set_runtime).
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct WasmRuntime;

impl AsyncRuntime for WasmRuntime {
    fn spawn(&self, future: BoxFuture<'static, ()>) {
        wasm_bindgen_futures::spawn_local(future);
    }

    fn spawn_blocking(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        // There is no blocking pool on Wasm: run the closure inline and hope it
        // is short enough not to be noticed.
        task();
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        let millis = u32::try_from(duration.as_millis()).unwrap_or_else(|_| {
            tracing::error!("Sleep duration too long, sleeping for u32::MAX ms");
            u32::MAX
        });

        Box::pin(gloo_timers::future::TimeoutFuture::new(millis))
    }
}
