// Copyright 2021 The Matrix.org Foundation C.I.C.
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

//! Spawning tasks on whichever async runtime the SDK was configured with.
//!
//! The runtime itself is described by [`crate::runtime::AsyncRuntime`]; this
//! module builds the joining and cancellation the SDK needs on top of the bare
//! "spawn and forget" it provides, so the semantics are identical no matter
//! which runtime is installed.

use std::{
    any::Any,
    fmt,
    future::Future,
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

use futures_channel::oneshot;
use futures_util::{
    FutureExt,
    future::{AbortHandle as RawAbortHandle, Abortable, Aborted},
};
/// The Tokio runtime handle, for callers that need to enter the SDK's runtime
/// context themselves.
///
/// Only available when the `tokio-runtime` feature is enabled; it says nothing
/// about which runtime was actually installed with
/// [`crate::runtime::set_runtime`].
#[cfg(all(not(target_family = "wasm"), feature = "tokio-runtime"))]
pub use tokio::runtime::{Handle, Runtime};

use crate::{SendOutsideWasm, runtime};

/// The reason a spawned task did not run to completion.
///
/// Returned when awaiting a [`JoinHandle`].
pub struct JoinError {
    repr: Repr,
}

enum Repr {
    /// The task was aborted, or dropped before it could complete.
    Cancelled,
    /// The task panicked, carrying the panic payload.
    Panic(Mutex<Box<dyn Any + Send + 'static>>),
}

impl JoinError {
    fn cancelled() -> Self {
        Self { repr: Repr::Cancelled }
    }

    fn panicked(payload: Box<dyn Any + Send + 'static>) -> Self {
        Self { repr: Repr::Panic(Mutex::new(payload)) }
    }

    /// Whether the error was caused by the task being cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self.repr, Repr::Cancelled)
    }

    /// Whether the error was caused by the task panicking.
    pub fn is_panic(&self) -> bool {
        matches!(self.repr, Repr::Panic(_))
    }

    /// Consume the error, returning the panic payload that caused it.
    ///
    /// # Panics
    ///
    /// Panics if the task was cancelled rather than panicking; check with
    /// [`JoinError::is_panic`] first.
    pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
        match self.repr {
            Repr::Panic(payload) => {
                payload.into_inner().expect("The panic payload mutex should never be poisoned")
            }
            Repr::Cancelled => panic!("`JoinError::into_panic()` called on a cancelled task"),
        }
    }
}

impl fmt::Debug for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Cancelled => f.write_str("JoinError::Cancelled"),
            Repr::Panic(_) => f.write_str("JoinError::Panic(..)"),
        }
    }
}

impl fmt::Display for JoinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Cancelled => f.write_str("task was cancelled"),
            Repr::Panic(_) => f.write_str("task panicked"),
        }
    }
}

impl std::error::Error for JoinError {}

/// A handle used to abort a spawned task, and to observe whether it is done.
///
/// Cloning it gives another handle to the same task.
#[derive(Debug, Clone)]
pub struct AbortHandle {
    inner: RawAbortHandle,
    finished: Arc<AtomicBool>,
}

impl AbortHandle {
    /// Abort the task, preventing it from being polled again.
    ///
    /// Does nothing if the task has already finished. A task that is
    /// mid-execution stops at its next `await` point.
    pub fn abort(&self) {
        self.inner.abort();
    }

    /// Whether the task has stopped running, be it because it completed,
    /// panicked or was aborted.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire) || self.inner.is_aborted()
    }

    /// Whether [`AbortHandle::abort`] was called on this task.
    pub fn is_aborted(&self) -> bool {
        self.inner.is_aborted()
    }
}

/// A handle to a task spawned with [`spawn`] or [`spawn_blocking`].
///
/// Awaiting it yields the task's output, or a [`JoinError`] if the task was
/// aborted or panicked. Dropping it does *not* stop the task; use
/// [`JoinHandle::abort`] or [`JoinHandleExt::abort_on_drop`] for that.
#[derive(Debug)]
pub struct JoinHandle<T> {
    receiver: oneshot::Receiver<Result<T, JoinError>>,
    abort_handle: AbortHandle,
}

impl<T> JoinHandle<T> {
    /// Abort the task, preventing it from being polled again.
    pub fn abort(&self) {
        self.abort_handle.abort();
    }

    /// Get a handle that can be used to abort the task later on.
    pub fn abort_handle(&self) -> AbortHandle {
        self.abort_handle.clone()
    }

    /// Whether the task has stopped running.
    pub fn is_finished(&self) -> bool {
        self.abort_handle.is_finished()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.get_mut().receiver).poll(context) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(outcome)) => Poll::Ready(outcome),
            // The sender was dropped without sending anything, which means the
            // task itself was dropped before it could complete.
            Poll::Ready(Err(oneshot::Canceled)) => Poll::Ready(Err(JoinError::cancelled())),
        }
    }
}

/// Spawn a future on the installed async runtime.
///
/// The task starts running in the background right away and keeps running even
/// if the returned [`JoinHandle`] is dropped.
///
/// Panics inside the task are caught and reported through the [`JoinHandle`]
/// rather than unwinding into the runtime.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + SendOutsideWasm + 'static,
    F::Output: SendOutsideWasm + 'static,
{
    let (sender, receiver) = oneshot::channel();
    let (raw_abort_handle, abort_registration) = RawAbortHandle::new_pair();
    let finished = Arc::new(AtomicBool::new(false));

    let task_finished = finished.clone();
    let future = Abortable::new(future, abort_registration);

    runtime::runtime().spawn(Box::pin(async move {
        let result = AssertUnwindSafe(future).catch_unwind().await;

        task_finished.store(true, Ordering::Release);

        let outcome = match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(Aborted)) => Err(JoinError::cancelled()),
            Err(payload) => Err(JoinError::panicked(payload)),
        };

        // The receiver may well have been dropped; nobody is listening then.
        let _ = sender.send(outcome);
    }));

    JoinHandle { receiver, abort_handle: AbortHandle { inner: raw_abort_handle, finished } }
}

/// Run a blocking closure somewhere it won't stall the async executor.
///
/// Use this for work that would otherwise block an executor thread for a long
/// time: filesystem access, cryptographic key derivation, and the like.
///
/// Aborting the returned [`JoinHandle`] only has an effect if the closure
/// hasn't started running yet; a blocking closure cannot be interrupted.
pub fn spawn_blocking<F, T>(task: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = oneshot::channel();
    let (raw_abort_handle, abort_registration) = RawAbortHandle::new_pair();
    let finished = Arc::new(AtomicBool::new(false));

    let task_finished = finished.clone();
    let abort_handle = AbortHandle { inner: raw_abort_handle, finished };
    let started_abort_handle = abort_handle.clone();

    runtime::runtime().spawn_blocking(Box::new(move || {
        // There's no way to interrupt a blocking closure, but if it hasn't
        // started yet an abort can still keep it from running at all.
        let outcome = if started_abort_handle.is_aborted() {
            Err(JoinError::cancelled())
        } else {
            match std::panic::catch_unwind(AssertUnwindSafe(task)) {
                Ok(value) => Ok(value),
                Err(payload) => Err(JoinError::panicked(payload)),
            }
        };

        task_finished.store(true, Ordering::Release);

        let _ = sender.send(outcome);
    }));

    // `Abortable` is unused here, but keeping the same handle type means callers
    // don't have to care whether they spawned async or blocking work.
    drop(abort_registration);

    JoinHandle { receiver, abort_handle }
}

/// Yield execution back to the runtime, so other tasks get a chance to run.
///
/// This is the runtime-agnostic equivalent of `tokio::task::yield_now`.
pub async fn yield_now() {
    /// A future that returns `Pending` exactly once, after waking itself.
    struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                return Poll::Ready(());
            }

            self.yielded = true;
            context.waker().wake_by_ref();

            Poll::Pending
        }
    }

    YieldNow { yielded: false }.await
}

/// A type ensuring a task is aborted on drop.
#[derive(Debug)]
pub struct AbortOnDrop<T>(JoinHandle<T>);

impl<T> AbortOnDrop<T> {
    pub fn new(join_handle: JoinHandle<T>) -> Self {
        Self(join_handle)
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl<T> Future for AbortOnDrop<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(context)
    }
}

/// Trait to create an [`AbortOnDrop`] from a [`JoinHandle`].
pub trait JoinHandleExt<T> {
    fn abort_on_drop(self) -> AbortOnDrop<T>;
}

impl<T> JoinHandleExt<T> for JoinHandle<T> {
    fn abort_on_drop(self) -> AbortOnDrop<T> {
        AbortOnDrop::new(self)
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;
    use matrix_sdk_test_macros::async_test;

    use super::{spawn, yield_now};

    #[async_test]
    async fn test_spawn() {
        let future = async { 42 };
        let join_handle = spawn(future);

        assert_matches!(join_handle.await, Ok(42));
    }

    #[async_test]
    async fn test_abort() {
        let future = std::future::pending::<()>();
        let join_handle = spawn(future);

        join_handle.abort();

        assert!(join_handle.await.unwrap_err().is_cancelled());
    }

    #[async_test]
    async fn test_panicking_task_is_reported_as_such() {
        let join_handle = spawn(async { panic!("this task is doomed") });

        let error = join_handle.await.unwrap_err();

        // Panics can't be caught on Wasm, where unwinding isn't supported.
        #[cfg(not(target_family = "wasm"))]
        assert!(error.is_panic(), "{error:?}");
        #[cfg(target_family = "wasm")]
        let _ = error;
    }

    #[async_test]
    async fn test_dropping_the_handle_does_not_abort_the_task() {
        let (sender, receiver) = futures_channel::oneshot::channel();

        drop(spawn(async move {
            let _ = sender.send(42);
        }));

        assert_matches!(receiver.await, Ok(42));
    }

    #[async_test]
    async fn test_yield_now() {
        yield_now().await;
    }

    #[cfg(not(target_family = "wasm"))]
    #[async_test]
    async fn test_spawn_blocking() {
        use super::spawn_blocking;

        assert_matches!(spawn_blocking(|| 42).await, Ok(42));
    }
}
