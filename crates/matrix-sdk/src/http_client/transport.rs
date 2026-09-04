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

//! The HTTP transport the SDK sends its requests over.

#[cfg(feature = "reqwest-transport")]
use std::sync::Arc;
use std::{fmt, time::Duration};

use bytes::Bytes;
use eyeball::SharedObservable;
use matrix_sdk_common::{BoxFuture, SendOutsideWasm, SyncOutsideWasm};

use super::TransmissionProgress;
use crate::error::HttpError;

/// Something that can send an HTTP request and return its response.
///
/// Every request the SDK makes — Matrix API calls, OAuth 2.0 exchanges, media
/// up- and downloads — goes through an implementation of this trait. The
/// default one uses [`reqwest`], which needs a Tokio runtime; implement this
/// trait over another HTTP client to run the SDK on another async runtime, and
/// hand it to
/// [`ClientBuilder::http_transport()`](crate::ClientBuilder::http_transport).
///
/// Retries, the request timeout policy and error classification stay in the
/// SDK: an implementation is only expected to send the request and give back
/// what the server said. Failures at the network layer should be reported as
/// [`HttpError::Transport`], which the SDK treats as worth retrying.
pub trait HttpSend: fmt::Debug + SendOutsideWasm + SyncOutsideWasm + 'static {
    /// Send the given request and wait for its response.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to send, with its body fully in memory.
    ///
    /// * `timeout` - How long to wait for the response before giving up, if the
    ///   transport supports it. `None` means no timeout.
    ///
    /// * `send_progress` - An observable to report upload progress into, by
    ///   adding the body's length to `total` and each sent chunk's length to
    ///   `current`. Transports that can't observe the body being sent can
    ///   ignore it.
    fn send_request<'a>(
        &'a self,
        request: http::Request<Bytes>,
        timeout: Option<Duration>,
        send_progress: SharedObservable<TransmissionProgress>,
    ) -> BoxFuture<'a, Result<http::Response<Bytes>, HttpError>>;
}

/// The [`HttpSend`] implementation backed by [`reqwest`].
#[cfg(feature = "reqwest-transport")]
#[derive(Clone, Debug)]
pub struct ReqwestTransport {
    inner: reqwest::Client,
}

#[cfg(feature = "reqwest-transport")]
impl ReqwestTransport {
    /// Create a transport sending its requests with the given
    /// [`reqwest::Client`].
    pub fn new(client: reqwest::Client) -> Self {
        Self { inner: client }
    }

    /// The `reqwest` client this transport sends its requests with.
    pub fn client(&self) -> &reqwest::Client {
        &self.inner
    }
}

#[cfg(feature = "reqwest-transport")]
impl From<reqwest::Client> for ReqwestTransport {
    fn from(client: reqwest::Client) -> Self {
        Self::new(client)
    }
}

#[cfg(feature = "reqwest-transport")]
impl HttpSend for ReqwestTransport {
    fn send_request<'a>(
        &'a self,
        request: http::Request<Bytes>,
        timeout: Option<Duration>,
        send_progress: SharedObservable<TransmissionProgress>,
    ) -> BoxFuture<'a, Result<http::Response<Bytes>, HttpError>> {
        Box::pin(super::execute_request(&self.inner, request, timeout, send_progress))
    }
}

/// Wrap a [`reqwest::Client`] into the transport the SDK uses by default.
#[cfg(feature = "reqwest-transport")]
pub(crate) fn reqwest_transport(client: reqwest::Client) -> Arc<dyn HttpSend> {
    Arc::new(ReqwestTransport::new(client))
}
