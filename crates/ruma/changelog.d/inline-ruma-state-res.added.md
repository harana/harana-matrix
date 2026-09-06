The vendored `ruma` now carries `state_res`, the state resolution and PDU
authorization rules, behind the `state-res` feature. `matrix-sdk-state-res`
asks for that feature, so without it no crate in the workspace resolved.
