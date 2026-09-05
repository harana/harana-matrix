`matrix-sdk` builds again for `wasm32-unknown-unknown` without the
`reqwest-transport` feature. `http_client::wasm` imported
`response_to_http_response` unconditionally, but that function is gated behind
`reqwest-transport`, so any wasm build with `js` alone failed with
`error[E0432]: unresolved import`. That import, and `std::time::Duration`
alongside it, are only needed by the `reqwest-transport`-gated
`execute_request`, so they are now gated the same way.
