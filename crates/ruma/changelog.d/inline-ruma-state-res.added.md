The vendored `ruma` now carries the two pieces the inlining left behind:
`api::appservice`, with the application service registration file types
(`Registration`, `RegistrationInit`, `Namespaces`, `Namespace`) behind the
`appservice-api-c` / `appservice-api-s` features, and `state_res`, the state
resolution and PDU authorization rules, behind the `state-res` feature.
`matrix-sdk-appservice` and `matrix-sdk-state-res` ask for those features, so
without them no crate in the workspace resolved.
