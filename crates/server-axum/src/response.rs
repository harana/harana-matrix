//! Turning Matrix response types into HTTP responses.

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use bytes::BytesMut;
use ruma::api::{OutgoingResponse, OutgoingResponseExt as _};

use crate::error::Error;

/// A wrapper turning the response type of a Matrix endpoint into an HTTP
/// response.
///
/// ```
/// use ruma::{api::client::account::whoami, owned_user_id};
/// use server_axum::{Ruma, RumaResponse};
///
/// async fn who_am_i(
///     _request: Ruma<whoami::v3::Request>,
/// ) -> RumaResponse<whoami::v3::Response> {
///     RumaResponse(whoami::v3::Response::new(
///         owned_user_id!("@alice:localhost"),
///         false,
///     ))
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RumaResponse<T>(pub T);

impl<T> RumaResponse<T> {
    /// Consume this wrapper, returning the response it wraps.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for RumaResponse<T>
where
    T: OutgoingResponse,
{
    fn from(response: T) -> Self {
        Self(response)
    }
}

impl<T> IntoResponse for RumaResponse<T>
where
    T: OutgoingResponse,
{
    fn into_response(self) -> Response {
        match self.0.try_into_http_response::<BytesMut>() {
            Ok(response) => response.map(|body| Body::from(body.freeze())).into_response(),
            Err(error) => {
                tracing::error!("failed to serialize a Matrix response: {error}");
                Error::unknown("The server failed to serialize its response").into_response()
            }
        }
    }
}
