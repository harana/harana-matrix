//! The extractor turning HTTP requests into Matrix request types.

use std::ops::{Deref, DerefMut};

use axum::{
    body::Bytes,
    extract::{
        FromRequest, FromRequestParts as _, RawPathParams, Request,
        rejection::{BytesRejection, RawPathParamsRejection},
    },
};
use http::StatusCode;
use ruma::api::{IncomingRequest, Metadata, error::ErrorKind, path_builder::PathBuilder as _};

use crate::error::Error;

/// An extractor deserializing a request into the Matrix request type `T`.
///
/// The endpoint a handler serves is derived from the `T` of its `Ruma<T>`
/// argument, so this is what ties a handler to a route:
///
/// ```
/// use ruma::api::client::session::get_login_types;
/// use server_axum::{MatrixRouter, Ruma, RumaResponse};
///
/// async fn login_types(
///     _request: Ruma<get_login_types::v3::Request>,
/// ) -> RumaResponse<get_login_types::v3::Response> {
///     RumaResponse(get_login_types::v3::Response::new(vec![]))
/// }
///
/// let router: axum::Router = MatrixRouter::new().handle(login_types).build();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Ruma<T>(pub T);

impl<T> Ruma<T> {
    /// Consume this extractor, returning the request it wraps.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Ruma<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Ruma<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S, T> FromRequest<S> for Ruma<T>
where
    T: IncomingRequest,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = request.into_parts();

        let mut path_args: Vec<String> =
            match RawPathParams::from_request_parts(&mut parts, state).await {
                Ok(params) => params.iter().map(|(_, value)| value.to_owned()).collect(),
                // A route that has no path parameter at all.
                Err(RawPathParamsRejection::MissingPathParams(_)) => Vec::new(),
                Err(rejection) => return Err(Error::invalid_param(rejection.body_text())),
            };

        // Endpoints whose last path parameter may be empty, like the state key of
        // `/_matrix/client/v3/rooms/{room_id}/state/{event_type}/{state_key}`, are also
        // served on the path without it. See
        // `MatrixRouter::empty_trailing_param_compat()`.
        if path_args.len() + 1 == path_parameter_count::<T>() {
            path_args.push(String::new());
        }

        let bytes = Bytes::from_request(Request::from_parts(parts.clone(), body), state)
            .await
            .map_err(body_rejection)?;

        T::try_from_http_request(http::Request::from_parts(parts, bytes), &path_args)
            .map(Self)
            .map_err(Error::from)
    }
}

/// The number of path parameters the paths of the endpoint `T` have.
///
/// Every path of an endpoint has the same parameters, so the first one is
/// representative.
fn path_parameter_count<T: Metadata>() -> usize {
    T::PATH_BUILDER
        .all_paths()
        .next()
        .map(|path| path.split('/').filter(|segment| segment.starts_with('{')).count())
        .unwrap_or(0)
}

/// Turn the rejection of the body extractor into a Matrix error.
fn body_rejection(rejection: BytesRejection) -> Error {
    let status_code = rejection.status();
    let kind = if status_code == StatusCode::PAYLOAD_TOO_LARGE {
        ErrorKind::TooLarge
    } else {
        ErrorKind::Unknown
    };

    Error::new(status_code, kind, rejection.body_text())
}
