`Client::http_client()` now returns `&Arc<dyn HttpSend>` instead of
`&reqwest::Client`, since the transport is no longer necessarily `reqwest`.

The OAuth 2.0 error types carry `HttpClientError<HttpError>` instead of
`HttpClientError<reqwest::Error>`, for the same reason, and `HttpError` has a
new `Transport` variant for failures reported by a custom transport.

`ClientBuilder::{http_client, proxy, user_agent, disable_ssl_verification,
add_root_certificates, disable_built_in_root_certificates}` now require the
`reqwest-transport` feature, as they configure the `reqwest` client the SDK
would otherwise build itself. The `rustls-aws-lc-rs`, `socks` and `testing`
features enable it, so builds that use any of those are unaffected.
