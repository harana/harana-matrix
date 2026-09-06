# server-axum

The complete route table of a Matrix homeserver, as an [axum] router.

This crate owns no server logic. It defines every route a homeserver has to
answer, taken straight from the endpoint metadata in [`ruma`], and gives you the
plumbing to attach your own handlers to them:

- [`MatrixRouter`] registers every client-server and server-server endpoint
  Ruma knows about, on every path each endpoint has ever had (unstable, legacy
  `r0`/`v1` and current), with the right HTTP method.
- Endpoints you have not implemented answer `404 M_UNRECOGNIZED`, and a known
  path queried with the wrong method answers `405 M_UNRECOGNIZED`, as the
  specification requires.
- [`Ruma`] is an extractor that turns an HTTP request into the Ruma request type
  of the endpoint; [`RumaResponse`] turns a Ruma response type back into an HTTP
  response.
- [`routes::all()`] describes every endpoint (name, method, paths,
  authentication scheme, rate limiting), which is handy for coverage reports
  and tests.

## Example

```rust
use ruma::api::client::{account::whoami, session::get_login_types};
use server_axum::{MatrixRouter, Ruma, RumaResponse};

async fn login_types(
    _request: Ruma<get_login_types::v3::Request>,
) -> RumaResponse<get_login_types::v3::Response> {
    RumaResponse(get_login_types::v3::Response::new(vec![]))
}

async fn who_am_i(
    request: Ruma<whoami::v3::Request>,
) -> Result<RumaResponse<whoami::v3::Response>, server_axum::Error> {
    let _ = request;
    Err(server_axum::Error::unrecognized())
}

let router: axum::Router = MatrixRouter::new().handle(login_types).handle(who_am_i).build();
```

The endpoint each handler serves is taken from the type of its [`Ruma`]
argument, so a handler can never be attached to the wrong route.

Handlers may take any other axum extractor before the [`Ruma`] one, including
[`State`](axum::extract::State), in which case the router is built for that
state type:

```rust
use axum::extract::State;
use ruma::api::client::discovery::get_capabilities;
use server_axum::{MatrixRouter, Ruma, RumaResponse};

#[derive(Clone)]
struct AppState;

async fn capabilities(
    State(_state): State<AppState>,
    _request: Ruma<get_capabilities::v3::Request>,
) -> RumaResponse<get_capabilities::v3::Response> {
    RumaResponse(get_capabilities::v3::Response::new(Default::default()))
}

let router: axum::Router =
    MatrixRouter::<AppState>::new().handle(capabilities).build().with_state(AppState);
```

## What is not here

Authentication, rate limiting, persistence and the actual Matrix logic. The
route table tells you which authentication scheme each endpoint uses (see
[`EndpointMeta::authentication`]); enforcing it is up to the server.

The endpoints are the ones the vendored Ruma defines, so endpoints the
specification has since removed, like the `v1` variants of `send_join` and
`send_leave`, are not among them.

[axum]: https://docs.rs/axum
