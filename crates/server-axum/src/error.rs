//! Matrix errors, and their conversion into HTTP responses.

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use bytes::BytesMut;
use http::StatusCode;
use ruma::api::{
    OutgoingResponseExt as _,
    error::{
        DeserializationError, Error as RumaError, ErrorBody, ErrorKind, FromHttpRequestError,
        HeaderDeserializationError, StandardErrorBody,
    },
};

/// A `Result` whose error is a Matrix [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error to answer a Matrix request with.
///
/// It renders as the [standard error response] of the Matrix APIs: a status
/// code, an `errcode` and a human-readable `error` message.
///
/// [standard error response]: https://spec.matrix.org/v1.19/client-server-api/#standard-error-response
#[derive(Clone, Debug)]
pub struct Error(RumaError);

impl Error {
    /// Construct an error with the given status code, error code and message.
    pub fn new(status_code: StatusCode, kind: ErrorKind, message: impl Into<String>) -> Self {
        Self(RumaError::new(
            status_code,
            ErrorBody::Standard(StandardErrorBody::new(kind, message.into())),
        ))
    }

    /// `404 M_UNRECOGNIZED`: the endpoint is not implemented by this server.
    pub fn unrecognized() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            ErrorKind::Unrecognized,
            "Unrecognized request: this endpoint is not implemented by this server",
        )
    }

    /// `405 M_UNRECOGNIZED`: the endpoint exists, but not for this HTTP method.
    pub fn method_not_allowed() -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            ErrorKind::Unrecognized,
            "Unrecognized request: this endpoint does not accept this HTTP method",
        )
    }

    /// `400 M_BAD_JSON`: the request is valid JSON, but not what the endpoint
    /// expects.
    pub fn bad_json(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorKind::BadJson, message)
    }

    /// `400 M_NOT_JSON`: the request body is not valid JSON.
    pub fn not_json(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorKind::NotJson, message)
    }

    /// `400 M_MISSING_PARAM`: a parameter the endpoint requires is missing.
    pub fn missing_param(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorKind::MissingParam, message)
    }

    /// `400 M_INVALID_PARAM`: a parameter of the request has an invalid value.
    pub fn invalid_param(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, ErrorKind::InvalidParam, message)
    }

    /// `500 M_UNKNOWN`: the server failed to handle the request.
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, ErrorKind::Unknown, message)
    }

    /// The status code of this error.
    pub fn status_code(&self) -> StatusCode {
        self.0.status_code
    }

    /// The `errcode` of this error, if it uses the standard error format.
    pub fn kind(&self) -> Option<&ErrorKind> {
        self.0.error_kind()
    }

    /// The Ruma error this wraps.
    pub fn as_ruma(&self) -> &RumaError {
        &self.0
    }

    /// Consume this error, returning the Ruma error it wraps.
    pub fn into_ruma(self) -> RumaError {
        self.0
    }
}

impl From<RumaError> for Error {
    fn from(error: RumaError) -> Self {
        Self(error)
    }
}

impl From<FromHttpRequestError> for Error {
    fn from(error: FromHttpRequestError) -> Self {
        match error {
            // The endpoint is served on this path, but not for the method the request used.
            FromHttpRequestError::MethodMismatch { .. } => Self::method_not_allowed(),
            FromHttpRequestError::Deserialization(error) => error.into(),
            _ => Self::bad_json(error.to_string()),
        }
    }
}

impl From<DeserializationError> for Error {
    fn from(error: DeserializationError) -> Self {
        match &error {
            // A body that is not valid JSON at all, as opposed to JSON that doesn't match what the
            // endpoint expects.
            DeserializationError::Json(json_error)
                if matches!(
                    json_error.classify(),
                    serde_json::error::Category::Syntax | serde_json::error::Category::Eof
                ) =>
            {
                Self::not_json(error.to_string())
            }
            DeserializationError::Utf8(_) => Self::not_json(error.to_string()),
            DeserializationError::Json(_) => Self::bad_json(error.to_string()),
            DeserializationError::Query(_) | DeserializationError::Ident(_) => {
                Self::invalid_param(error.to_string())
            }
            DeserializationError::Header(HeaderDeserializationError::MissingHeader(_)) => {
                Self::missing_param(error.to_string())
            }
            DeserializationError::Header(_) => Self::invalid_param(error.to_string()),
            _ => Self::bad_json(error.to_string()),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        into_response(self.0)
    }
}

/// Turn a Ruma error into an HTTP response.
///
/// Serializing a standard error body cannot fail, so the fallback below is only
/// there to avoid a panic if Ruma ever changes that.
fn into_response(error: RumaError) -> Response {
    let status_code = error.status_code;

    match error.try_into_http_response::<BytesMut>() {
        Ok(response) => response.map(|body| Body::from(body.freeze())).into_response(),
        Err(error) => {
            tracing::error!("failed to serialize a Matrix error into a response: {error}");
            (status_code, [(http::header::CONTENT_TYPE, "application/json")], FALLBACK_BODY)
                .into_response()
        }
    }
}

/// The body used when an error cannot be serialized.
const FALLBACK_BODY: &str = r#"{"errcode":"M_UNKNOWN","error":"Internal server error"}"#;
