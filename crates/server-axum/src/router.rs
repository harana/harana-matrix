//! The router holding the routes of a Matrix homeserver.

use std::{any::TypeId, borrow::Cow, collections::BTreeMap};

use axum::{
    Router,
    routing::{MethodFilter, MethodRouter},
};
use http::Method;
use ruma::api::{Metadata, path_builder::PathBuilder as _};

use crate::{endpoint::AuthSchemeKind, error::Error, handler::RumaHandler, routes};

/// The path suffixes whose last parameter the specification allows to be empty.
///
/// The state key of a state event may be the empty string, in which case
/// clients and servers may omit it from the path entirely, with or without a
/// trailing slash. Ruma only knows the path with the parameter, so the other
/// two are registered as aliases of it.
const OPTIONAL_TRAILING_PARAMS: &[&str] = &["/state/{event_type}/{state_key}"];

/// A single route: one HTTP method on one path.
struct Route<S> {
    /// The method this route answers.
    method: Method,

    /// The handler of the route.
    handler: MethodRouter<S>,

    /// The name of the endpoint this route belongs to.
    endpoint: &'static str,

    /// Whether the handler is still the stub answering `404 M_UNRECOGNIZED`.
    stub: bool,
}

/// A registered endpoint.
struct Endpoint {
    /// The name of the endpoint.
    name: &'static str,

    /// Whether a handler was attached to it.
    handled: bool,
}

/// The routes of a Matrix homeserver.
///
/// A new router knows every client-server and server-server endpoint Ruma
/// defines, on every path each of them has ever had, and answers all of them
/// with `404 M_UNRECOGNIZED`. Attaching a handler with
/// [`handle()`](Self::handle) replaces that stub for one endpoint, and
/// [`build()`](Self::build) turns the result into an [`axum::Router`].
///
/// ```
/// use ruma::api::client::discovery::get_supported_versions;
/// use server_axum::{MatrixRouter, Ruma, RumaResponse};
///
/// async fn versions(
///     _request: Ruma<get_supported_versions::Request>,
/// ) -> RumaResponse<get_supported_versions::Response> {
///     RumaResponse(get_supported_versions::Response::new(vec![
///         "v1.19".to_owned(),
///     ]))
/// }
///
/// let router: axum::Router = MatrixRouter::new().handle(versions).build();
/// ```
pub struct MatrixRouter<S = ()> {
    /// The routes, by path.
    routes: BTreeMap<Cow<'static, str>, Vec<Route<S>>>,

    /// The endpoints that were registered, by the type of their request.
    endpoints: BTreeMap<TypeId, Endpoint>,

    /// Whether to serve the endpoints of [`OPTIONAL_TRAILING_PARAMS`] on their
    /// shorter paths too.
    empty_trailing_param_compat: bool,
}

impl<S> MatrixRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Create a router with a stub route for every known endpoint.
    pub fn new() -> Self {
        let mut router = Self::empty();
        routes::register_all(&mut router);
        router
    }

    /// Create a router without any route.
    ///
    /// Endpoints that are not given a handler answer `404 M_UNRECOGNIZED`, same
    /// as with [`new()`](Self::new), but a path that is only known through
    /// its stub answers `404` instead of `405` when queried with the wrong
    /// HTTP method.
    pub fn empty() -> Self {
        Self {
            routes: BTreeMap::new(),
            endpoints: BTreeMap::new(),
            empty_trailing_param_compat: true,
        }
    }

    /// Serve the endpoint of `handler` with it.
    ///
    /// The endpoint is the one of the [`Ruma<R>`](crate::Ruma) argument of the
    /// handler, so a handler cannot be attached to the wrong route. It
    /// replaces any handler the endpoint already had.
    ///
    /// # Panics
    ///
    /// A few endpoints share a path and a method with another endpoint, and are
    /// told apart by a query parameter: the delayed events of [MSC4140] are
    /// sent to the same path as ordinary message and state events. Giving a
    /// handler to both endpoints of such a pair panics, since only one of
    /// them can answer; use [`handle_raw()`](Self::handle_raw) with a handler
    /// that dispatches between the two instead.
    ///
    /// [MSC4140]: https://github.com/matrix-org/matrix-spec-proposals/pull/4140
    pub fn handle<H, T>(mut self, handler: H) -> Self
    where
        H: RumaHandler<S, T>,
        T: 'static,
    {
        self.insert::<H::Endpoint>(handler.into_method_router());
        self
    }

    /// Serve the endpoint `R` with the given axum method router.
    ///
    /// This is the escape hatch for endpoints that need more than the
    /// [`Ruma`](crate::Ruma) extractor can offer, like streaming the body
    /// of a media upload instead of buffering it, or answering two
    /// endpoints that share a route. The method router is registered on every
    /// path of the endpoint, and it is up to it to answer only the HTTP
    /// methods the endpoints use.
    pub fn handle_raw<R>(mut self, method_router: MethodRouter<S>) -> Self
    where
        R: Metadata + 'static,
    {
        self.insert::<R>(method_router);
        self
    }

    /// Whether to also serve the endpoints whose last path parameter may be
    /// empty on the path without it.
    ///
    /// The state key of a state event may be empty, in which case the state
    /// endpoints are queried without it, with or without a trailing slash.
    /// Ruma only defines the path that has the state key, so those two are
    /// registered as aliases of it, and the [`Ruma`](crate::Ruma) extractor
    /// passes an empty state key to the endpoint.
    ///
    /// This is on by default.
    pub fn empty_trailing_param_compat(mut self, enabled: bool) -> Self {
        self.empty_trailing_param_compat = enabled;
        self
    }

    /// The names of the endpoints that have a handler.
    pub fn handled_endpoints(&self) -> Vec<&'static str> {
        self.endpoint_names(true)
    }

    /// The names of the endpoints that are still answered by the stub.
    ///
    /// This is the coverage report of a server: every endpoint listed here
    /// answers `404 M_UNRECOGNIZED`.
    pub fn unhandled_endpoints(&self) -> Vec<&'static str> {
        self.endpoint_names(false)
    }

    /// Turn this into an axum router.
    ///
    /// Unknown paths answer `404 M_UNRECOGNIZED`. A known path queried with an
    /// HTTP method the endpoint does not have answers `405` with the same error
    /// code, as the specification requires.
    pub fn build(self) -> Router<S> {
        let mut routes = self.routes;

        if self.empty_trailing_param_compat {
            add_empty_trailing_param_aliases(&mut routes);
        }

        let mut router = Router::new();

        for (path, path_routes) in routes {
            let mut method_router = MethodRouter::new();

            for route in path_routes {
                method_router = method_router.merge(route.handler);
            }

            router = router
                .route(&path, method_router.fallback(|| async { Error::method_not_allowed() }));
        }

        router.fallback(|| async { Error::unrecognized() })
    }

    /// Register the stub answering `404 M_UNRECOGNIZED` for the endpoint `R`.
    pub(crate) fn register_stub<R>(&mut self, name: &'static str)
    where
        R: Metadata + 'static,
        R::Authentication: AuthSchemeKind,
    {
        self.endpoints.insert(TypeId::of::<R>(), Endpoint { name, handled: false });

        let stub = axum::routing::on(method_filter::<R>(), || async { Error::unrecognized() });
        self.insert_routes::<R>(stub, name, true);
    }

    /// Register the given method router as the handler of the endpoint `R`.
    fn insert<R>(&mut self, method_router: MethodRouter<S>)
    where
        R: Metadata + 'static,
    {
        let name = match self.endpoints.get_mut(&TypeId::of::<R>()) {
            Some(endpoint) => {
                endpoint.handled = true;
                endpoint.name
            }
            // An endpoint of a router built with `empty()`, or one this crate doesn't know about.
            None => UNKNOWN_ENDPOINT,
        };

        self.insert_routes::<R>(method_router, name, false);
    }

    /// Register the given method router on every path of the endpoint `R`.
    fn insert_routes<R>(&mut self, method_router: MethodRouter<S>, name: &'static str, stub: bool)
    where
        R: Metadata,
    {
        for path in R::PATH_BUILDER.all_paths() {
            let path_routes = self.routes.entry(Cow::Borrowed(path)).or_default();
            let route =
                Route { method: R::METHOD, handler: method_router.clone(), endpoint: name, stub };

            match path_routes.iter().position(|other| other.method == route.method) {
                Some(index) => {
                    let previous = &path_routes[index];

                    assert!(
                        stub || previous.stub || previous.endpoint == name,
                        "`{name}` and `{}` are both served on `{} {path}`, so they cannot both \
                         have a handler; use `handle_raw()` with a handler that dispatches \
                         between them",
                        previous.endpoint,
                        route.method,
                    );

                    path_routes[index] = route;
                }
                None => path_routes.push(route),
            }
        }
    }

    /// The names of the endpoints that do, or do not, have a handler.
    fn endpoint_names(&self, handled: bool) -> Vec<&'static str> {
        let mut names: Vec<_> = self
            .endpoints
            .values()
            .filter(|endpoint| endpoint.handled == handled)
            .map(|endpoint| endpoint.name)
            .collect();

        names.sort_unstable();
        names
    }
}

impl<S> Default for MatrixRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// The name given to the routes of endpoints this crate doesn't know about.
const UNKNOWN_ENDPOINT: &str = "<unknown>";

/// The axum method filter matching the HTTP method of the endpoint `R`.
///
/// Endpoints using `GET` also answer `HEAD` requests, which axum handles on its
/// own.
pub(crate) fn method_filter<R: Metadata>() -> MethodFilter {
    MethodFilter::try_from(R::METHOD)
        .unwrap_or_else(|_| panic!("`{}` is not an HTTP method axum can route", R::METHOD))
}

/// Serve the endpoints whose last path parameter may be empty on the path
/// without it too.
fn add_empty_trailing_param_aliases<S>(routes: &mut BTreeMap<Cow<'static, str>, Vec<Route<S>>>) {
    let aliases: Vec<_> = routes
        .iter()
        .filter(|(path, _)| OPTIONAL_TRAILING_PARAMS.iter().any(|suffix| path.ends_with(suffix)))
        .flat_map(|(path, path_routes)| {
            let base = path.rsplit_once('/').expect("the path has several segments").0.to_owned();
            let with_slash = format!("{base}/");

            [base, with_slash].into_iter().flat_map(move |alias| {
                path_routes.iter().map(move |route| {
                    (
                        Cow::Owned(alias.clone()),
                        route.method.clone(),
                        route.handler.clone(),
                        route.endpoint,
                        route.stub,
                    )
                })
            })
        })
        .collect();

    for (path, method, handler, endpoint, stub) in aliases {
        let path_routes = routes.entry(path).or_default();

        if path_routes.iter().any(|route| route.method == method) {
            continue;
        }

        path_routes.push(Route { method, handler, endpoint, stub });
    }
}
