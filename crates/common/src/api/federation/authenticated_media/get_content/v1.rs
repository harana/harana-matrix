//! `/v1/` ([spec])
//!
//! [spec]: https://spec.matrix.org/v1.19/server-server-api/#get_matrixfederationv1mediadownloadmediaid

use std::time::Duration;

use crate::{
    __ruma::{api::request, metadata},
    api::federation::{
        authenticated_media::{ContentMetadata, FileOrLocation, ResponseBody},
        authentication::ServerSignatures,
    },
};

metadata! {
    method: GET,
    rate_limited: true,
    authentication: ServerSignatures,
    path: "/_matrix/federation/v1/media/download/{media_id}",
}

/// Request type for the `get_content` endpoint.
#[request]
pub struct Request {
    /// The media ID from the `mxc://` URI (the path component).
    #[ruma_api(path)]
    pub media_id: String,

    /// The maximum duration that the client is willing to wait to start
    /// receiving data, in the case that the content has not yet been
    /// uploaded.
    ///
    /// The default value is 20 seconds.
    #[ruma_api(query)]
    #[serde(
        with = "crate::__ruma::serde::duration::ms",
        default = "crate::__ruma::media::default_download_timeout",
        skip_serializing_if = "crate::__ruma::media::is_default_download_timeout"
    )]
    pub timeout_ms: Duration,
}

impl Request {
    /// Creates a new `Request` with the given media ID.
    pub fn new(media_id: String) -> Self {
        Self { media_id, timeout_ms: crate::__ruma::media::default_download_timeout() }
    }
}

/// Response type for the `get_content` endpoint.
#[derive(Debug, Clone)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Response {
    /// The metadata of the media.
    pub metadata: ContentMetadata,

    /// The content of the media.
    pub content: FileOrLocation,
}

impl Response {
    /// Creates a new `Response` with the given metadata and content.
    pub fn new(metadata: ContentMetadata, content: FileOrLocation) -> Self {
        Self { metadata, content }
    }
}

#[cfg(feature = "client")]
impl crate::__ruma::api::IncomingResponse for Response {
    type EndpointError = crate::__ruma::api::error::Error;

    fn try_from_http_response_inner(
        http_response: http::Response<&[u8]>,
    ) -> Result<Self, crate::__ruma::api::error::DeserializationError> {
        let ResponseBody { metadata, content, .. } =
            ResponseBody::try_from_http_response(http_response)?;
        Ok(Self { metadata, content })
    }
}

#[cfg(feature = "server")]
impl crate::__ruma::api::OutgoingResponse for Response {
    type Body = ResponseBody;

    fn try_into_http_response_inner(
        self,
    ) -> Result<http::Response<Self::Body>, crate::__ruma::api::error::IntoHttpError> {
        ResponseBody::new(self.metadata, self.content).try_into_http_response()
    }
}
