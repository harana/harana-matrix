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

//! A pluggable abstraction over the async runtime the SDK runs on.
//!
//! The SDK itself never talks to a concrete async runtime. Everything it needs
//! from one — spawning a task, running a blocking closure somewhere else, and
//! sleeping — is described by the [`AsyncRuntime`] trait. A single
//! implementation is installed process-wide with [`set_runtime`], and the rest
//! of the SDK goes through [`crate::executor`], [`crate::sleep`] and
//! [`crate::timeout`], which are built on top of it.
//!
//! # Which runtime is used?
//!
//! * If [`set_runtime`] was called before the first SDK API that needs the
//!   runtime, that implementation is used.
//! * Otherwise, on Wasm, tasks are spawned on the JS event loop through
//!   `wasm-bindgen-futures`.
//! * Otherwise, if the `tokio-runtime` feature is enabled (it is by default),
//!   Tokio is used, and the calling code must be running inside a Tokio
//!   runtime.
//! * Otherwise, the first use panics with a message telling the caller to
//!   install a runtime.
//!
//! # Using another runtime
//!
//! Any executor can be plugged in by implementing [`AsyncRuntime`] and
//! installing it before the SDK is used. For instance, with [compio]:
//!
//! ```ignore
//! use std::{sync::Arc, time::Duration};
//!
//! use sdk_common::{
//!     BoxFuture,
//!     runtime::{AsyncRuntime, set_runtime},
//! };
//!
//! #[derive(Debug)]
//! struct CompioRuntime;
//!
//! impl AsyncRuntime for CompioRuntime {
//!     fn spawn(&self, future: BoxFuture<'static, ()>) {
//!         compio::runtime::spawn(future).detach();
//!     }
//!
//!     fn spawn_blocking(&self, task: Box<dyn FnOnce() + Send + 'static>) {
//!         compio::runtime::spawn_blocking(task).detach();
//!     }
//!
//!     fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
//!         // compio's timers are `!Send`, so run one as a task of its own and
//!         // wait for it through a channel, which is `Send`.
//!         let (sender, receiver) = futures_channel::oneshot::channel();
//!
//!         compio::runtime::spawn(async move {
//!             compio::time::sleep(duration).await;
//!             let _ = sender.send(());
//!         })
//!         .detach();
//!
//!         Box::pin(async move {
//!             let _ = receiver.await;
//!         })
//!     }
//! }
//!
//! compio::runtime::Runtime::new().unwrap().block_on(async {
//!     set_runtime(Arc::new(CompioRuntime)).unwrap();
//!     // … use the SDK here …
//! });
//! ```
//!
//! Note the `Send` bounds: the SDK spawns and awaits `Send` futures
//! throughout, so a thread-per-core runtime whose own futures are `!Send`
//! needs a bridge like the one above wherever it hands a future back.
//!
//! [compio]: https://docs.rs/compio

use std::{fmt, sync::Arc, time::Duration};

use crate::{BoxFuture, SendOutsideWasm, SyncOutsideWasm};

#[cfg(all(not(target_family = "wasm"), feature = "tokio-runtime"))]
mod tokio_runtime;
#[cfg(target_family = "wasm")]
mod wasm_runtime;

#[cfg(all(not(target_family = "wasm"), feature = "tokio-runtime"))]
pub use tokio_runtime::TokioRuntime;
#[cfg(target_family = "wasm")]
pub use wasm_runtime::WasmRuntime;

/// The parts of an async runtime the SDK depends on.
///
/// See the [module documentation](self) for how to install an implementation.
///
/// Implementations are expected to be cheap to call and to never block the
/// caller.
pub trait AsyncRuntime: fmt::Debug + SendOutsideWasm + SyncOutsideWasm + 'static {
    /// Spawn a future, running it to completion in the background.
    ///
    /// The future is detached: there is no handle to it, and it must keep
    /// running until it resolves. Cancellation and joining are layered on top
    /// of this by [`crate::executor::spawn`], so implementations don't need to
    /// provide them.
    fn spawn(&self, future: BoxFuture<'static, ()>);

    /// Run a closure that may block for a long time, off the async executor.
    ///
    /// Implementations that have no notion of a blocking pool (Wasm, most
    /// notably) may run the closure inline, at the cost of stalling the
    /// executor.
    fn spawn_blocking(&self, task: Box<dyn FnOnce() + Send + 'static>);

    /// Return a future that resolves after (at least) `duration`.
    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()>;
}

/// Error returned by [`set_runtime`] when a runtime has already been installed.
///
/// The runtime can only be installed once, and only before the first SDK API
/// that spawns a task or sleeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRuntimeError;

impl fmt::Display for SetRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an async runtime has already been installed for this process")
    }
}

impl std::error::Error for SetRuntimeError {}

/// Install the async runtime the SDK will use.
///
/// This must be called before any other SDK API that spawns a task or sleeps,
/// and can only be called once; later calls return [`SetRuntimeError`].
///
/// Not calling it at all is fine as long as one of the built-in runtimes is
/// available; see the [module documentation](self).
pub fn set_runtime(runtime: Arc<dyn AsyncRuntime>) -> Result<(), SetRuntimeError> {
    imp::set(runtime)
}

/// Get the async runtime the SDK is using, installing the default one if
/// [`set_runtime`] hasn't been called.
///
/// # Panics
///
/// Panics if no runtime was installed and the crate was built without a
/// built-in runtime (that is, on a non-Wasm target with the `tokio-runtime`
/// feature disabled).
pub fn runtime() -> Arc<dyn AsyncRuntime> {
    imp::get()
}

#[allow(unreachable_code)]
fn default_runtime() -> Arc<dyn AsyncRuntime> {
    #[cfg(target_family = "wasm")]
    return Arc::new(WasmRuntime);

    #[cfg(all(not(target_family = "wasm"), feature = "tokio-runtime"))]
    return Arc::new(TokioRuntime);

    panic!(
        "no async runtime is available: the `tokio-runtime` feature of \
         `sdk-common` is disabled, so `sdk_common::runtime::set_runtime()` \
         must be called before using the SDK"
    )
}

#[cfg(not(target_family = "wasm"))]
mod imp {
    use std::sync::{Arc, OnceLock};

    use super::{AsyncRuntime, SetRuntimeError, default_runtime};

    static RUNTIME: OnceLock<Arc<dyn AsyncRuntime>> = OnceLock::new();

    pub(super) fn set(runtime: Arc<dyn AsyncRuntime>) -> Result<(), SetRuntimeError> {
        RUNTIME.set(runtime).map_err(|_| SetRuntimeError)
    }

    pub(super) fn get() -> Arc<dyn AsyncRuntime> {
        RUNTIME.get_or_init(default_runtime).clone()
    }
}

// On Wasm the runtime is not `Send`/`Sync`, so it can't live in a `static`;
// it's stored per-thread instead. Wasm is single-threaded in practice, and
// every thread that uses the SDK gets its own copy of the default runtime.
#[cfg(target_family = "wasm")]
mod imp {
    use std::{cell::RefCell, sync::Arc};

    use super::{AsyncRuntime, SetRuntimeError, default_runtime};

    thread_local! {
        static RUNTIME: RefCell<Option<Arc<dyn AsyncRuntime>>> = const { RefCell::new(None) };
    }

    pub(super) fn set(runtime: Arc<dyn AsyncRuntime>) -> Result<(), SetRuntimeError> {
        RUNTIME.with_borrow_mut(|slot| {
            if slot.is_some() {
                return Err(SetRuntimeError);
            }
            *slot = Some(runtime);
            Ok(())
        })
    }

    pub(super) fn get() -> Arc<dyn AsyncRuntime> {
        RUNTIME.with_borrow_mut(|slot| slot.get_or_insert_with(default_runtime).clone())
    }
}
