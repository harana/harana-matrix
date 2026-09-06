//! Types and traits for working with the [Matrix](https://matrix.org) protocol.
//!
//! This crate is the facade over a vendored and trimmed-down fork of
//! [ruma](https://github.com/ruma/ruma). Each of the upstream crates is a crate of this
//! workspace, re-exported here under the module name upstream uses:
//!
//! | upstream crate         | crate here       | module here          |
//! | ---------------------- | ---------------- | -------------------- |
//! | `ruma-common`          | `common`         | crate root           |
//! | `ruma-events`          | `events`         | [`events`]           |
//! | `ruma-client-api`      | `client-api`     | [`api::client`]      |
//! | `ruma-federation-api`  | `federation-api` | `api::federation`    |
//! | `ruma-appservice-api`  | `appservice-api` | [`api::appservice`]  |
//! | `ruma-html`            | `html`           | `html`               |
//! | `ruma-signatures`      | `signatures`     | `signatures`         |
//! | `ruma-state-res`       | `state-res`      | [`state_res`]        |
//!
//! Only the parts used by this workspace are kept: of the appservice API only the registration
//! file format ([`api::appservice`]) is vendored, and the identity-service and push-gateway APIs
//! are not vendored at all.
//!
//! > For internal consistency, Ruma uses American spelling for variable names. Names may differ
//! > in the serialized representation, as the Matrix specification has a mix of British and
//! > American English.
//!
//! # Cargo features
//!
//! * `client` -- `OutgoingRequest` / `IncomingResponse` impls, for code talking *to* a server.
//! * `server` -- `IncomingRequest` / `OutgoingResponse` impls, for code answering requests.
//! * `appservice-api` -- the application service registration types ([`api::appservice`]). Also
//!   available as `appservice-api-c` and `appservice-api-s`, for parity with upstream `ruma`.
//! * `federation-api` -- the server-server API (`api::federation`). Implies `signatures`.
//! * `signatures` -- digital signatures (`signatures`).
//! * `state-res` -- state resolution and the PDU authorization rules ([`state_res`]). Implies
//!   `signatures`.
//! * `html` -- HTML parsing and sanitizing (`html`).
//! * `markdown` -- parse markdown to construct messages.
//! * `rand` -- generate random identifiers.
//! * `js` -- randomness and current system time in browser environments.
//! * `unstable-uniffi` -- UniFFI bindings for _some_ types. Work in progress.
//! * `compat-*` -- tolerate known, reasonable deviations from the spec in external data. They
//!   never make this crate *produce* non-spec-compliant data.
//! * `unstable-mscXXXX` -- upcoming Matrix features that may change or be removed. Using any of
//!   them opts you out of all semver guarantees.
//!
//! # Compile-time `cfg` settings
//!
//! These are read from environment variables by `build.rs`:
//!
//! * `RUMA_IDENTIFIERS_STORAGE=Arc` -- back owned identifier types with `Arc<str>` instead of
//!   `Box<str>`.
//! * `RUMA_UNSTABLE_EXHAUSTIVE_TYPES` -- compile all types as exhaustive. Opts you out of all
//!   semver guarantees.

#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use common::*;
#[doc(inline)]
pub use ::events;
#[cfg(feature = "html")]
#[doc(inline)]
pub use ::html;
#[cfg(feature = "signatures")]
#[doc(inline)]
pub use ::signatures;
#[cfg(feature = "state-res")]
#[doc(inline)]
pub use ::state_res;

/// Core types used to define the requests and responses for each endpoint in the various
/// [Matrix API specifications][apis], and the endpoints themselves.
///
/// [apis]: https://spec.matrix.org/v1.19/#matrix-apis
pub mod api {
    pub use common::api::*;

    #[cfg(feature = "appservice-api")]
    #[doc(inline)]
    pub use ::appservice_api as appservice;
    #[cfg(any(feature = "client", feature = "server"))]
    #[doc(inline)]
    pub use ::client_api as client;
    #[cfg(all(feature = "federation-api", any(feature = "client", feature = "server")))]
    #[doc(inline)]
    pub use ::federation_api as federation;
}
