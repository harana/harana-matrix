//! Tests of the route table itself: that it can be built, and that it answers
//! the way the specification requires.

use std::collections::BTreeMap;

use axum::{Router, body::Body, extract::State};
use http::{Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use ruma::{
    api::{
        client::{
            membership::joined_rooms, session::get_login_types, state::get_state_event_for_key,
        },
        error::ErrorKind,
    },
    owned_room_id,
};
use server_axum::{Api, AuthKind, MatrixRouter, Ruma, RumaResponse, routes};
use tower::ServiceExt as _;

/// Send a request to the given router and return its status code and body.
async fn call(router: Router, method: Method, uri: &str) -> (StatusCode, serde_json::Value) {
    let request =
        Request::builder().method(method).uri(uri).body(Body::empty()).expect("a valid request");
    let response = router.oneshot(request).await.expect("the router is infallible");
    let status_code = response.status();
    let bytes = response.into_body().collect().await.expect("the body is in memory").to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status_code, body)
}

#[test]
fn every_endpoint_is_registered() {
    // This is the test that matters most: building the router panics if two
    // endpoints claim the same path with incompatible parameter names, so it
    // checks the whole route table at once.
    let router = MatrixRouter::new();
    let endpoints = routes::all();

    assert_eq!(router.handled_endpoints(), Vec::<&str>::new());
    assert_eq!(router.unhandled_endpoints().len(), endpoints.len());

    let _: Router = router.build();
}

#[test]
fn endpoints_cover_both_apis() {
    let endpoints = routes::all();

    assert!(endpoints.iter().any(|endpoint| endpoint.api == Api::ClientServer));
    assert!(endpoints.iter().any(|endpoint| endpoint.api == Api::Federation));

    // Every path is one axum can route, and every endpoint has at least one.
    for endpoint in &endpoints {
        assert!(!endpoint.paths.is_empty(), "{} has no path", endpoint.name);

        for path in &endpoint.paths {
            assert!(path.starts_with('/'), "the path `{path}` of {} is relative", endpoint.name);
        }
    }
}

#[test]
fn endpoint_metadata_matches_the_specification() {
    let endpoints = routes::all();

    let login_types = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "client::session::get_login_types::v3")
        .expect("the endpoint is known");

    assert_eq!(login_types.api, Api::ClientServer);
    assert_eq!(login_types.method, Method::GET);
    assert_eq!(login_types.authentication, AuthKind::NoAccessToken);
    assert_eq!(login_types.latest_path(), "/_matrix/client/v3/login");

    let transactions = endpoints
        .iter()
        .find(|endpoint| endpoint.name == "federation::transactions::send_transaction_message::v1")
        .expect("the endpoint is known");

    assert_eq!(transactions.api, Api::Federation);
    assert_eq!(transactions.method, Method::PUT);
    assert_eq!(transactions.authentication, AuthKind::ServerSignatures);
    assert!(transactions.authentication.is_required());
}

#[tokio::test]
async fn an_unknown_path_is_unrecognized() {
    let router = MatrixRouter::new().build();

    let (status_code, body) = call(router, Method::GET, "/_matrix/client/v3/nope").await;

    assert_eq!(status_code, StatusCode::NOT_FOUND);
    assert_eq!(body["errcode"], "M_UNRECOGNIZED");
}

#[tokio::test]
async fn an_endpoint_without_a_handler_is_unrecognized() {
    let router = MatrixRouter::new().build();

    let (status_code, body) = call(router, Method::GET, "/_matrix/client/v3/login").await;

    assert_eq!(status_code, StatusCode::NOT_FOUND);
    assert_eq!(body["errcode"], "M_UNRECOGNIZED");
}

#[tokio::test]
async fn a_known_path_with_the_wrong_method_is_not_allowed() {
    let router = MatrixRouter::new().build();

    let (status_code, body) = call(router, Method::DELETE, "/_matrix/client/v3/login").await;

    assert_eq!(status_code, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["errcode"], "M_UNRECOGNIZED");
}

#[tokio::test]
async fn a_handler_answers_every_path_of_its_endpoint() {
    async fn login_types(
        _request: Ruma<get_login_types::v3::Request>,
    ) -> RumaResponse<get_login_types::v3::Response> {
        RumaResponse(get_login_types::v3::Response::new(vec![]))
    }

    let router = MatrixRouter::new().handle(login_types);

    assert_eq!(router.handled_endpoints(), vec!["client::session::get_login_types::v3"]);

    let router = router.build();

    // The current path, and the one it had before Matrix 1.1.
    for path in ["/_matrix/client/v3/login", "/_matrix/client/r0/login"] {
        let (status_code, body) = call(router.clone(), Method::GET, path).await;

        assert_eq!(status_code, StatusCode::OK, "{path} did not answer");
        assert_eq!(body["flows"], serde_json::json!([]));
    }
}

#[tokio::test]
async fn a_handler_receives_the_path_parameters() {
    async fn joined_rooms(
        _request: Ruma<joined_rooms::v3::Request>,
    ) -> RumaResponse<joined_rooms::v3::Response> {
        RumaResponse(joined_rooms::v3::Response::new(vec![owned_room_id!("!room:localhost")]))
    }

    async fn state_event(
        request: Ruma<get_state_event_for_key::v3::Request>,
    ) -> RumaResponse<get_state_event_for_key::v3::Response> {
        // Echo the parameters back so the test can check they were parsed.
        let content = serde_json::json!({
            "room_id": request.room_id.as_str(),
            "event_type": request.event_type.to_string(),
            "state_key": request.state_key,
        });

        RumaResponse(get_state_event_for_key::v3::Response::new(
            serde_json::from_value(content).expect("the content is an object"),
        ))
    }

    let router = MatrixRouter::new().handle(joined_rooms).handle(state_event).build();

    let (status_code, body) = call(
        router.clone(),
        Method::GET,
        "/_matrix/client/v3/rooms/!room:localhost/state/m.room.name/",
    )
    .await;

    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(body["room_id"], "!room:localhost");
    assert_eq!(body["event_type"], "m.room.name");
    assert_eq!(body["state_key"], "");

    // The path with the state key omitted entirely answers the same way.
    let (status_code, body) = call(
        router.clone(),
        Method::GET,
        "/_matrix/client/v3/rooms/!room:localhost/state/m.room.name",
    )
    .await;

    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(body["state_key"], "");

    let (status_code, body) = call(
        router,
        Method::GET,
        "/_matrix/client/v3/rooms/!room:localhost/state/m.room.member/@alice:localhost",
    )
    .await;

    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(body["state_key"], "@alice:localhost");
}

#[tokio::test]
async fn an_invalid_path_parameter_is_rejected() {
    async fn joined_members(
        _request: Ruma<ruma::api::client::membership::joined_members::v3::Request>,
    ) -> RumaResponse<ruma::api::client::membership::joined_members::v3::Response> {
        RumaResponse(ruma::api::client::membership::joined_members::v3::Response::new(
            Default::default(),
        ))
    }

    let router = MatrixRouter::new().handle(joined_members).build();

    // `not-a-room-id` is not a room ID, so the request never reaches the handler.
    let (status_code, body) =
        call(router, Method::GET, "/_matrix/client/v3/rooms/not-a-room-id/joined_members").await;

    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(body["errcode"], "M_INVALID_PARAM");
}

#[tokio::test]
async fn a_handler_can_take_the_state_of_the_router() {
    #[derive(Clone)]
    struct AppState {
        flows: Vec<get_login_types::v3::LoginType>,
    }

    async fn login_types(
        State(state): State<AppState>,
        _request: Ruma<get_login_types::v3::Request>,
    ) -> RumaResponse<get_login_types::v3::Response> {
        RumaResponse(get_login_types::v3::Response::new(state.flows))
    }

    let router = MatrixRouter::<AppState>::new()
        .handle(login_types)
        .build()
        .with_state(AppState { flows: vec![] });

    let (status_code, _) = call(router, Method::GET, "/_matrix/client/v3/login").await;

    assert_eq!(status_code, StatusCode::OK);
}

#[test]
fn the_error_kinds_are_the_ones_the_specification_defines() {
    let error = server_axum::Error::unrecognized();

    assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
    assert_eq!(error.kind(), Some(&ErrorKind::Unrecognized));
}

#[cfg(feature = "unstable-msc4140")]
#[test]
#[should_panic(expected = "cannot both have a handler")]
fn two_endpoints_that_share_a_route_cannot_both_be_handled() {
    use ruma::api::client::{delayed_events::delayed_message_event, message::send_message_event};

    async fn send(
        _request: Ruma<send_message_event::v3::Request>,
    ) -> RumaResponse<send_message_event::v3::Response> {
        RumaResponse(send_message_event::v3::Response::new(ruma::owned_event_id!("$id:localhost")))
    }

    async fn send_delayed(
        _request: Ruma<delayed_message_event::unstable::Request>,
    ) -> RumaResponse<delayed_message_event::unstable::Response> {
        RumaResponse(delayed_message_event::unstable::Response::new("delay-id".to_owned()))
    }

    // Delayed events are sent to the path of ordinary events, with a `delay` query
    // parameter, so only one handler can own that route.
    let _: MatrixRouter = MatrixRouter::new().handle(send).handle(send_delayed);
}

#[tokio::test]
async fn the_empty_state_key_aliases_can_be_turned_off() {
    async fn state_event(
        _request: Ruma<get_state_event_for_key::v3::Request>,
    ) -> RumaResponse<get_state_event_for_key::v3::Response> {
        RumaResponse(get_state_event_for_key::v3::Response::new(
            serde_json::from_value(serde_json::json!({})).expect("the content is an object"),
        ))
    }

    let router = MatrixRouter::new().empty_trailing_param_compat(false).handle(state_event).build();

    let (status_code, body) =
        call(router, Method::GET, "/_matrix/client/v3/rooms/!room:localhost/state/m.room.name")
            .await;

    assert_eq!(status_code, StatusCode::NOT_FOUND);
    assert_eq!(body["errcode"], "M_UNRECOGNIZED");
}

#[test]
fn the_endpoints_that_share_a_route_are_the_known_ones() {
    // Delayed events are the only endpoints the specification serves on the route
    // of another endpoint, telling the two apart by a query parameter. Any
    // other pair showing up here would silently shadow one of the two
    // endpoints, so it has to be handled explicitly.
    let mut routes: BTreeMap<(String, &str), Vec<&str>> = BTreeMap::new();

    for endpoint in routes::all() {
        for path in &endpoint.paths {
            routes.entry((endpoint.method.to_string(), path)).or_default().push(endpoint.name);
        }
    }

    let shared: Vec<_> = routes
        .into_iter()
        .filter(|(_, endpoints)| endpoints.len() > 1)
        .map(|((method, path), endpoints)| (format!("{method} {path}"), endpoints))
        .collect();

    let expected = if cfg!(feature = "unstable-msc4140") {
        vec![
            (
                "PUT /_matrix/client/v3/rooms/{room_id}/send/{event_type}/{txn_id}".to_owned(),
                vec![
                    "client::delayed_events::delayed_message_event::unstable",
                    "client::message::send_message_event::v3",
                ],
            ),
            (
                "PUT /_matrix/client/v3/rooms/{room_id}/state/{event_type}/{state_key}".to_owned(),
                vec![
                    "client::delayed_events::delayed_state_event::unstable",
                    "client::state::send_state_event::v3",
                ],
            ),
        ]
    } else {
        vec![]
    };

    assert_eq!(shared, expected);
}
