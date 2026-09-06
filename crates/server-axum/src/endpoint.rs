//! Descriptions of the endpoints a Matrix homeserver serves.

use http::Method;
use ruma::api::{
    Metadata,
    auth_scheme::{
        AccessToken, AccessTokenOptional, AppserviceToken, AppserviceTokenOptional, AuthScheme,
        NoAccessToken, NoAuthentication,
    },
    federation::authentication::ServerSignatures,
    path_builder::PathBuilder as _,
};

/// The Matrix API an endpoint belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Api {
    /// The [client-server API], which clients use to talk to their homeserver.
    ///
    /// This includes the media repository and the client-side server discovery
    /// endpoints.
    ///
    /// [client-server API]: https://spec.matrix.org/v1.19/client-server-api/
    ClientServer,

    /// The [server-server API], which homeservers use to talk to each other.
    ///
    /// This includes the server key and server discovery endpoints.
    ///
    /// [server-server API]: https://spec.matrix.org/v1.19/server-server-api/
    Federation,
}

impl Api {
    /// The name of this API, as used in the specification.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClientServer => "client-server",
            Self::Federation => "server-server",
        }
    }
}

/// The authentication an endpoint expects.
///
/// A server has to enforce this itself: this crate only reports what the
/// specification requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AuthKind {
    /// The endpoint is public.
    None,

    /// The endpoint must be queried without an access token.
    NoAccessToken,

    /// The endpoint requires an access token.
    AccessToken,

    /// The endpoint accepts an optional access token, and behaves differently
    /// with one.
    AccessTokenOptional,

    /// The endpoint requires an appservice access token.
    AppserviceToken,

    /// The endpoint accepts either a user or an appservice access token.
    AppserviceTokenOptional,

    /// The endpoint requires a signature of the request by the sending server.
    ServerSignatures,
}

impl AuthKind {
    /// Whether an endpoint with this authentication requires some form of
    /// credentials.
    pub fn is_required(self) -> bool {
        matches!(self, Self::AccessToken | Self::AppserviceToken | Self::ServerSignatures)
    }
}

/// An [`AuthScheme`] that this crate can name.
///
/// This is implemented for every authentication scheme Ruma defines.
pub trait AuthSchemeKind: AuthScheme {
    /// The [`AuthKind`] matching this authentication scheme.
    const KIND: AuthKind;
}

macro_rules! impl_auth_scheme_kind {
    ( $( $ty:ty => $kind:ident ),* $(,)? ) => {
        $(
            impl AuthSchemeKind for $ty {
                const KIND: AuthKind = AuthKind::$kind;
            }
        )*
    };
}

impl_auth_scheme_kind! {
    NoAuthentication => None,
    NoAccessToken => NoAccessToken,
    AccessToken => AccessToken,
    AccessTokenOptional => AccessTokenOptional,
    AppserviceToken => AppserviceToken,
    AppserviceTokenOptional => AppserviceTokenOptional,
    ServerSignatures => ServerSignatures,
}

/// The description of a single endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct EndpointMeta {
    /// The name of the endpoint, which is the path of its Ruma module.
    ///
    /// For instance `client::session::login::v3` for the endpoint defined by
    /// `ruma::api::client::session::login::v3`.
    pub name: &'static str,

    /// The API the endpoint belongs to.
    pub api: Api,

    /// The HTTP method of the endpoint.
    ///
    /// Endpoints using `GET` also answer `HEAD` requests.
    pub method: Method,

    /// Every path the endpoint is served on, unstable and legacy paths
    /// included.
    ///
    /// Path parameters use the `{name}` syntax, which is also the one axum
    /// uses.
    pub paths: Vec<&'static str>,

    /// The authentication the endpoint expects.
    pub authentication: AuthKind,

    /// Whether the specification expects the endpoint to be rate limited.
    pub rate_limited: bool,
}

impl EndpointMeta {
    /// Build the metadata of the endpoint whose request type is `R`.
    pub fn of<R>(name: &'static str, api: Api) -> Self
    where
        R: Metadata,
        R::Authentication: AuthSchemeKind,
    {
        Self {
            name,
            api,
            method: R::METHOD,
            paths: R::PATH_BUILDER.all_paths().collect(),
            authentication: <R::Authentication as AuthSchemeKind>::KIND,
            rate_limited: R::RATE_LIMITED,
        }
    }

    /// The path the endpoint is served on by the most recent Matrix version
    /// that has it.
    ///
    /// This is the last of [`paths`](Self::paths), which is the only field this
    /// crate can derive it from; endpoints that only ever had unstable
    /// paths return that unstable path.
    pub fn latest_path(&self) -> &'static str {
        self.paths.last().copied().expect("every endpoint has at least one path")
    }
}
