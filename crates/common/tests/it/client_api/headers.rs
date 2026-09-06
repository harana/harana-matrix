#![cfg(feature = "client")]

use http::HeaderMap;
use harana_matrix_common::api::client::discovery::discover_homeserver;
use harana_matrix_common::api::{OutgoingRequestExt as _, auth_scheme::SendAccessToken};

#[test]
fn get_request_headers() {
    let req: http::Request<Vec<u8>> = discover_homeserver::Request::new()
        .try_into_http_request("https://homeserver.tld", SendAccessToken::None, ())
        .unwrap();

    assert_eq!(*req.headers(), HeaderMap::default());
}
