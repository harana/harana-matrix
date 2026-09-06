#![cfg(feature = "client")]

use harana_matrix_common::api::{
    OutgoingRequestExt as _, auth_scheme::SendAccessToken, client::discovery::discover_homeserver,
};
use http::HeaderMap;

#[test]
fn get_request_headers() {
    let req: http::Request<Vec<u8>> = discover_homeserver::Request::new()
        .try_into_http_request("https://homeserver.tld", SendAccessToken::None, ())
        .unwrap();

    assert_eq!(*req.headers(), HeaderMap::default());
}
