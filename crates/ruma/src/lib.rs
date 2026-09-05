//! Types and traits for working with the [Matrix](https://matrix.org) protocol.
//!
//! This crate is a vendored and trimmed-down fork of [ruma](https://github.com/ruma/ruma),
//! merged into a single crate. The upstream `ruma-common`, `ruma-events`, `ruma-client-api`,
//! `ruma-federation-api`, `ruma-html` and `ruma-signatures` crates are modules here:
//!
//! | upstream crate         | module here          |
//! | ---------------------- | -------------------- |
//! | `ruma-common`          | crate root           |
//! | `ruma-events`          | [`events`]           |
//! | `ruma-client-api`      | [`api::client`]      |
//! | `ruma-federation-api`  | `api::federation`    |
//! | `ruma-html`            | `html`               |
//! | `ruma-signatures`      | `signatures`         |
//!
//! Only the parts used by this workspace are kept: the appservice, identity-service and
//! push-gateway APIs and the state resolution implementation are not vendored.
//!
//! > For internal consistency, Ruma uses American spelling for variable names. Names may differ
//! > in the serialized representation, as the Matrix specification has a mix of British and
//! > American English.
//!
//! # Cargo features
//!
//! * `client` -- `OutgoingRequest` / `IncomingResponse` impls, for code talking *to* a server.
//! * `server` -- `IncomingRequest` / `OutgoingResponse` impls, for code answering requests.
//! * `federation-api` -- the server-server API ([`api::federation`]). Implies `signatures`.
//! * `signatures` -- digital signatures (`signatures`).
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

#![recursion_limit = "1024"]
#![warn(missing_docs)]
// https://github.com/rust-lang/rust-clippy/issues/9029
#![allow(clippy::derive_partial_eq_without_eq)]
// Upstream ruma style; `&[] as &[u8]` in tests reads better than the alternatives.
#![allow(trivial_casts)]
// Upstream ruma allows this; most `new()`s here take required arguments in other endpoints.
#![allow(clippy::new_without_default)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Lets this crate's own procedural macros resolve the `::ruma` paths they generate.
extern crate self as ruma;

#[cfg(feature = "unstable-uniffi")]
uniffi::setup_scaffolding!();

pub mod api;
pub mod authentication;
pub mod canonical_json;
pub mod directory;
pub mod encryption;
pub mod events;
pub mod http_headers;
#[cfg(feature = "html")]
pub mod html;
mod identifiers;
pub mod media;
mod percent_encode;
pub mod power_levels;
pub mod presence;
mod priv_owned_str;
pub mod profile;
pub mod push;
pub mod room;
pub mod room_version_rules;
pub mod serde;
#[cfg(feature = "signatures")]
pub mod signatures;
#[cfg(feature = "state-res")]
pub mod state_res;
pub mod third_party_invite;
pub mod thirdparty;
mod timestamp;
pub mod to_device;

#[doc(no_inline)]
pub use assign::assign;
#[doc(no_inline)]
pub use js_int::{Int, UInt, int, uint};
#[doc(no_inline)]
pub use js_option::JsOption;
#[cfg(feature = "unstable-msc4334")]
#[doc(no_inline)]
pub use language_tags::LanguageTag;
pub use web_time as time;

pub use self::{
    canonical_json::{CanonicalJsonError, CanonicalJsonObject, CanonicalJsonValue},
    identifiers::*,
    timestamp::{MilliSecondsSinceUnixEpoch, SecondsSinceUnixEpoch},
};

priv_owned_str!(uniffi);

/// Re-exports used by macro-generated code.
///
/// It is not considered part of this module's public API.
#[doc(hidden)]
pub mod exports {
    pub use bytes;
    pub use http;
    pub use ruma_macros;
    pub use serde;
    pub use serde_html_form;
    pub use serde_json;
}
