//! `GET /_matrix/static/client/login/` ([spec])
//!
//! Get login fallback web page.
//!
//! [spec]: https://spec.matrix.org/v1.19/client-server-api/#login-fallback

use crate::__ruma::{
    OwnedDeviceId,
    api::{auth_scheme::NoAccessToken, request},
    metadata,
};

metadata! {
    method: GET,
    rate_limited: false,
    authentication: NoAccessToken,
    path: "/_matrix/static/client/login/",
}

/// Request type for the `login_fallback` endpoint.
#[request]
#[derive(Default)]
pub struct Request {
    /// ID of the client device.
    #[ruma_api(query)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<OwnedDeviceId>,

    /// A display name to assign to the newly-created device.
    ///
    /// Ignored if `device_id` corresponds to a known device.
    #[ruma_api(query)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_device_display_name: Option<String>,
}

impl Request {
    /// Creates a new `Request` with the given auth type and session ID.
    pub fn new(
        device_id: Option<OwnedDeviceId>,
        initial_device_display_name: Option<String>,
    ) -> Self {
        Self { device_id, initial_device_display_name }
    }
}

/// Response type for the `login_fallback` endpoint.
#[derive(Debug, Clone)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Response {
    /// HTML to return to client.
    pub body: Vec<u8>,
}

impl Response {
    /// Creates a new `Response` with the given HTML body.
    pub fn new(body: Vec<u8>) -> Self {
        Self { body }
    }
}

#[cfg(feature = "server")]
impl crate::__ruma::api::OutgoingResponse for Response {
    type Body = crate::__ruma::api::BytesBody;

    fn try_into_http_response_inner(
        self,
    ) -> Result<http::Response<Self::Body>, crate::__ruma::api::error::IntoHttpError> {
        Ok(http::Response::builder()
            .status(http::StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(crate::__ruma::api::BytesBody(self.body))?)
    }
}

#[cfg(feature = "client")]
impl crate::__ruma::api::IncomingResponse for Response {
    type EndpointError = crate::__ruma::api::error::Error;

    fn try_from_http_response_inner(
        response: http::Response<&[u8]>,
    ) -> Result<Self, crate::__ruma::api::error::DeserializationError> {
        let body = response.into_body().to_owned();
        Ok(Self { body })
    }
}
