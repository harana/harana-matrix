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

//! Running blocking operations on an object from async code.
//!
//! This is a replacement for `deadpool_sync::SyncWrapper`, which can only
//! offload blocking work to Tokio or async-std. This one goes through
//! [`matrix_sdk_common::executor::spawn_blocking`] instead, so it follows
//! whichever runtime the SDK was configured with.

use std::{
    any::Any,
    fmt,
    sync::{Arc, Mutex},
};

use matrix_sdk_common::executor::spawn_blocking;

/// Possible errors returned when [`SyncWrapper::interact()`] fails.
#[derive(Debug)]
pub enum InteractError {
    /// The provided callback panicked.
    Panic(Box<dyn Any + Send + 'static>),

    /// The callback was cancelled, which happens when the wrapper was dropped
    /// while the callback was queued.
    Cancelled,
}

impl fmt::Display for InteractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panic(_) => f.write_str("Panic"),
            Self::Cancelled => f.write_str("Cancelled"),
        }
    }
}

impl std::error::Error for InteractError {}

/// A wrapper around an object whose operations block, so that they can be
/// called from async code without stalling the executor.
///
/// Access to the wrapped object goes through [`SyncWrapper::interact()`].
#[must_use]
pub struct SyncWrapper<T>
where
    T: Send + 'static,
{
    obj: Arc<Mutex<Option<T>>>,
}

impl<T> fmt::Debug for SyncWrapper<T>
where
    T: fmt::Debug + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncWrapper").field("obj", &self.obj).finish()
    }
}

impl<T> SyncWrapper<T>
where
    T: Send + 'static,
{
    /// Create a new wrapped object, running the (blocking) constructor off the
    /// executor.
    ///
    /// # Panics
    ///
    /// Panics if the constructor itself panics.
    pub async fn new<F, E>(f: F) -> Result<Self, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        E: Send + 'static,
    {
        let result = spawn_blocking(f)
            .await
            .expect("Creating the wrapped object should never panic or be cancelled");

        result.map(|obj| Self { obj: Arc::new(Mutex::new(Some(obj))) })
    }

    /// Interact with the underlying object.
    ///
    /// The closure runs on a thread where blocking is acceptable, so the async
    /// executor isn't held up while it runs.
    pub async fn interact<F, R>(&self, f: F) -> Result<R, InteractError>
    where
        F: FnOnce(&mut T) -> R + Send + 'static,
        R: Send + 'static,
    {
        let obj = self.obj.clone();
        let span = tracing::Span::current();

        spawn_blocking(move || {
            let mut guard = obj.lock().expect("The object mutex has been poisoned");
            let obj: &mut T = guard.as_mut().ok_or(InteractError::Cancelled)?;

            let _span = span.enter();

            Ok(f(obj))
        })
        .await
        .map_err(|error| {
            if error.is_panic() {
                InteractError::Panic(error.into_panic())
            } else {
                InteractError::Cancelled
            }
        })?
    }

    /// Whether the underlying mutex has been poisoned, which happens when a
    /// panic occurs while interacting with the object.
    pub fn is_mutex_poisoned(&self) -> bool {
        self.obj.is_poisoned()
    }
}

impl<T> Drop for SyncWrapper<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        let obj = self.obj.clone();

        // Dropping the wrapped object can block (closing an SQLite connection
        // flushes to disk), so get it off the executor too.
        drop(spawn_blocking(move || match obj.lock() {
            Ok(mut guard) => drop(guard.take()),
            Err(error) => drop(error.into_inner().take()),
        }));
    }
}
