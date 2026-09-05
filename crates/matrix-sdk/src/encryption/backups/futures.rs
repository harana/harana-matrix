// Copyright 2023 The Matrix.org Foundation C.I.C.
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

//! Named futures for the backup support.

use std::{future::IntoFuture, pin::Pin, time::Duration};

use futures_core::Stream;
use futures_util::StreamExt;
use matrix_sdk_common::boxed_into_future;
use thiserror::Error;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tracing::trace;

use super::{Backups, UploadState};
use crate::utils::ChannelObservable;

/// Error describing the ways that waiting for the backup upload to settle down
/// can fail.
#[derive(Clone, Copy, Debug, Error)]
pub enum SteadyStateError {
    /// The currently active backup got either deleted or a new one was created.
    ///
    /// No further room keys will be uploaded to the currently active
    /// backup.
    #[error("The backup got disabled while waiting for the room keys to be uploaded.")]
    BackupDisabled,
    /// Uploading the room keys to the homeserver failed due to a network error.
    ///
    /// Uploading will be retried again at a later point in time, or
    /// immediately if you wait for the steady state again.
    #[error("There was a network connection error.")]
    Connection,
    /// We missed some updates to the [`UploadState`] from the upload task.
    ///
    /// This error doesn't imply that there was an error with the uploading of
    /// room keys, it just means that we didn't receive all the transitions
    /// in the [`UploadState`]. You might want to retry waiting for the
    /// steady state.
    #[error("We couldn't read status updates from the upload task quickly enough.")]
    Lagged,
}

/// Named future for the [`Backups::wait_for_upload()`] method.
pub struct WaitForSteadyState<'a> {
    pub(super) backups: &'a Backups,
    pub(super) progress: ChannelObservable<UploadState>,
    pub(super) timeout: Option<Duration>,
    /// Should awaiting this future wake the upload task up, or only observe an
    /// upload which somebody else started?
    pub(super) trigger_upload: bool,
    /// The progress subscription, taken when this future was created rather
    /// than when it is awaited.
    ///
    /// This is what lets a caller trigger the upload between creating the
    /// future and awaiting it without the completion going unnoticed.
    pub(super) progress_stream:
        Pin<Box<dyn Stream<Item = Result<UploadState, BroadcastStreamRecvError>> + Send>>,
    /// The upload delay which was in place before [`Self::with_delay`]
    /// overrode it, to be restored once we are done waiting.
    pub(super) old_delay: Option<Duration>,
}

#[cfg(not(tarpaulin_include))]
impl std::fmt::Debug for WaitForSteadyState<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitForSteadyState")
            .field("timeout", &self.timeout)
            .field("trigger_upload", &self.trigger_upload)
            .finish_non_exhaustive()
    }
}

impl WaitForSteadyState<'_> {
    /// Subscribe to the progress of the backup upload step while waiting for it
    /// to settle down.
    pub fn subscribe_to_progress(
        &self,
    ) -> impl Stream<Item = Result<UploadState, BroadcastStreamRecvError>> + use<> {
        self.progress.subscribe()
    }

    /// Set the delay between each upload request.
    ///
    /// Uploading room keys might require multiple requests to be sent out. The
    /// [`Client`] waits for a while before it sends the next request out.
    ///
    /// This method allows you to override how long the [`Client`] will wait.
    /// The default value is 100 ms.
    ///
    /// The delay takes effect immediately, so that it also applies to an upload
    /// triggered before this future is awaited, and is restored once the future
    /// completes.
    ///
    /// [`Client`]: crate::Client
    pub fn with_delay(mut self, delay: Duration) -> Self {
        let mut lock = self.backups.client.inner.e2ee.backup_state.upload_delay.write().unwrap();

        self.timeout = Some(delay);
        self.old_delay = Some(std::mem::replace(&mut *lock, delay));

        drop(lock);

        self
    }
}

impl<'a> IntoFuture for WaitForSteadyState<'a> {
    type Output = Result<(), SteadyStateError>;
    boxed_into_future!(extra_bounds: 'a);

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let Self {
                backups,
                timeout: _,
                progress: _,
                trigger_upload,
                mut progress_stream,
                old_delay,
            } = self;

            // The stream replays the state as it was when this future was created, which
            // may well be the `Done` of an upload that finished earlier and which
            // therefore says nothing about the room keys we are waiting for. Take it out
            // of the way so that only later states can end the wait.
            let replayed_state = progress_stream.next().await;

            trace!("Waiting for the upload steady state");

            let ret = if !backups.are_enabled().await {
                Err(SteadyStateError::BackupDisabled)
            } else if !trigger_upload
                && matches!(replayed_state, Some(Ok(UploadState::Done | UploadState::Idle)))
                && !backups.has_room_keys_to_upload().await.unwrap_or(true)
            {
                // Nothing is in flight and there is nothing left to upload, so we are in
                // the steady state already. Without this, an observing wait would sit
                // there waiting for an upload which nobody is going to start.
                trace!("Every room key is backed up already");

                Ok(())
            } else {
                if trigger_upload {
                    backups.trigger_upload();
                }

                let mut ret = Ok(());

                // TODO: Do we want to be smart here and remember the count when we started
                // waiting and prevent the total from increasing, in case new room
                // keys arrive after we started waiting.
                while let Some(state) = progress_stream.next().await {
                    trace!(?state, "Update state while waiting for the backup steady state");

                    match state {
                        Ok(UploadState::Done) => {
                            ret = Ok(());
                            break;
                        }
                        Ok(UploadState::Error) => {
                            if backups.are_enabled().await {
                                ret = Err(SteadyStateError::Connection);
                            } else {
                                ret = Err(SteadyStateError::BackupDisabled);
                            }

                            break;
                        }
                        Err(_) => {
                            ret = Err(SteadyStateError::Lagged);
                            break;
                        }
                        _ => (),
                    }
                }

                ret
            };

            if let Some(old_delay) = old_delay {
                let mut lock = backups.client.inner.e2ee.backup_state.upload_delay.write().unwrap();
                *lock = old_delay;
            }

            ret
        })
    }
}
