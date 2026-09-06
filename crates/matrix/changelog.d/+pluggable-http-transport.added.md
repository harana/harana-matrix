The HTTP transport is now pluggable. Implement the new `HttpSend` trait over
any HTTP client and pass it to `ClientBuilder::http_transport()`, and every
request the SDK makes — Matrix API calls, OAuth 2.0 exchanges, media transfers
— goes through it. `reqwest` remains the default, behind the new, default-on
`reqwest-transport` feature.

Together with `sdk_common::runtime::set_runtime()`, this makes it
possible to run the SDK on an async runtime other than Tokio; see "Running on
another async runtime" in the crate documentation.
