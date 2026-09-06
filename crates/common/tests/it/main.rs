// The vendored ruma test suite needs the whole feature set:
//     cargo test -p ruma --features full
#![cfg(feature = "full")]
#![allow(unreachable_pub)]

mod api;
mod canonical_json;
mod client_api;
mod events;
mod federation_api;
mod html;
mod identifiers;
mod serde;
mod signatures;
