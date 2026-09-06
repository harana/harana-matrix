//! `POST /_matrix/client/*/logout/all`
//!
//! Invalidates all access tokens for a user, so that they can no longer be used for authorization.

pub mod v3 {
    //! `/v3/` ([spec])
    //!
    //! [spec]: https://spec.matrix.org/v1.19/client-server-api/#post_matrixclientv3logoutall

    #[cfg(feature = "client")]
    use crate::__ruma::api::EmptyBody;
    use crate::__ruma::{
        api::{auth_scheme::AccessToken, error::Error, response},
        metadata,
    };

    metadata! {
        method: POST,
        rate_limited: false,
        authentication: AccessToken,
        history: {
            1.0 => "/_matrix/client/r0/logout/all",
            1.1 => "/_matrix/client/v3/logout/all",
        }
    }

    /// Request type for the `logout_all` endpoint.
    #[derive(Debug, Clone, Default)]
    #[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
    pub struct Request {}

    impl Request {
        /// Creates an empty `Request`.
        pub fn new() -> Self {
            Self {}
        }
    }

    #[cfg(feature = "client")]
    impl crate::__ruma::api::OutgoingRequest for Request {
        type Body = EmptyBody;
        type EndpointError = Error;
        type IncomingResponse = Response;

        fn try_into_http_request_inner(
            self,
            base_url: &str,
            considering: std::borrow::Cow<'_, crate::__ruma::api::SupportedVersions>,
        ) -> Result<http::Request<EmptyBody>, crate::__ruma::api::error::IntoHttpError> {
            use crate::__ruma::api::Metadata;

            let url = Self::make_endpoint_url(considering, base_url, &[], "")?;

            let http_request =
                http::Request::builder().method(Self::METHOD).uri(url).body(EmptyBody)?;

            Ok(http_request)
        }
    }

    #[cfg(feature = "server")]
    impl crate::__ruma::api::IncomingRequest for Request {
        type EndpointError = Error;
        type OutgoingResponse = Response;

        fn try_from_http_request<B, S>(
            request: http::Request<B>,
            _path_args: &[S],
        ) -> Result<Self, crate::__ruma::api::error::FromHttpRequestError>
        where
            B: AsRef<[u8]>,
            S: AsRef<str>,
        {
            Self::check_request_method(request.method())?;

            Ok(Self {})
        }
    }

    /// Response type for the `logout_all` endpoint.
    #[response]
    #[derive(Default)]
    pub struct Response {}

    impl Response {
        /// Creates an empty `Response`.
        pub fn new() -> Self {
            Self {}
        }
    }
}
