//! Types, traits and cryptography for working with the [Matrix](https://matrix.org) protocol.
//!
//! This crate holds everything the client and the server halves of this
//! workspace share. It is made of two vendored forks:
//!
//! * a trimmed-down fork of [ruma], whose crates are the modules named after
//!   them: the identifiers, (de)serialization helpers, push rules, canonical
//!   JSON and the core request/response traits of [`api`] live at the crate
//!   root (upstream `ruma-common`), and the rest under the module upstream
//!   uses.
//!
//!   | upstream crate         | module here          |
//!   | ---------------------- | -------------------- |
//!   | `ruma-common`          | crate root           |
//!   | `ruma-events`          | [`events`]           |
//!   | `ruma-client-api`      | [`api::client`]      |
//!   | `ruma-federation-api`  | `api::federation`    |
//!   | `ruma-appservice-api`  | [`api::appservice`]  |
//!   | `ruma-html`            | `html`               |
//!   | `ruma-signatures`      | `signatures`         |
//!   | `ruma-state-res`       | [`state_res`]        |
//!
//!   Only the parts used by this workspace are kept: of the appservice API only
//!   the registration file format ([`api::appservice`]) is vendored, and the
//!   identity-service and push-gateway APIs are not vendored at all.
//!
//! * a fork of [vodozemac], the Olm and Megolm implementations, in `olm`.
//!
//! [ruma]: https://github.com/ruma/ruma
//! [vodozemac]: https://github.com/matrix-org/vodozemac
//!
//! > For internal consistency, Ruma uses American spelling for variable names.
//! > Names may differ
//! > in the serialized representation, as the Matrix specification has a mix of
//! > British and
//! > American English.
//!
//! # Cargo features
//!
//! * `client` -- `OutgoingRequest` / `IncomingResponse` impls, for code talking
//!   *to* a server.
//! * `server` -- `IncomingRequest` / `OutgoingResponse` impls, for code
//!   answering requests.
//! * `appservice-api` -- the application service registration types
//!   ([`api::appservice`]). Also available as `appservice-api-c` and
//!   `appservice-api-s`, for parity with upstream `ruma`.
//! * `federation-api` -- the server-server API (`api::federation`). Implies
//!   `signatures`.
//! * `signatures` -- digital signatures (`signatures`).
//! * `state-res` -- state resolution and the PDU authorization rules
//!   ([`state_res`]). Implies `signatures`.
//! * `html` -- HTML parsing and sanitizing (`html`).
//! * `olm` -- the Olm and Megolm ratchets (`olm`).
//! * `markdown` -- parse markdown to construct messages.
//! * `rand` -- generate random identifiers.
//! * `js` -- randomness and current system time in browser environments.
//! * `testing` -- test helpers (`testing`).
//! * `unstable-uniffi` -- UniFFI bindings for _some_ types. Work in progress.
//! * `compat-*` -- tolerate known, reasonable deviations from the spec in
//!   external data. They never make this crate *produce* non-spec-compliant
//!   data.
//! * `unstable-mscXXXX` -- upcoming Matrix features that may change or be
//!   removed. Using any of them opts you out of all semver guarantees.
//!
//! # Compile-time `cfg` settings
//!
//! These are read from environment variables by `build.rs`:
//!
//! * `RUMA_IDENTIFIERS_STORAGE=Arc` -- back owned identifier types with
//!   `Arc<str>` instead of `Box<str>`.
//! * `RUMA_UNSTABLE_EXHAUSTIVE_TYPES` -- compile all types as exhaustive. Opts
//!   you out of all semver guarantees.

#![recursion_limit = "1024"]
#![warn(missing_docs)]
// https://github.com/rust-lang/rust-clippy/issues/9029
#![allow(clippy::derive_partial_eq_without_eq)]
// Upstream ruma style; `&[] as &[u8]` in tests reads better than the alternatives.
#![allow(trivial_casts)]
// Upstream ruma allows this; most `new()`s here take required arguments in other endpoints.
#![allow(clippy::new_without_default)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Lets this crate's own procedural macros resolve the paths they generate.
extern crate self as harana_matrix_common;

/// The paths the vendored ruma modules used when they were one `ruma` crate.
///
/// Not part of the public API.
#[doc(hidden)]
pub use crate as __ruma;

#[cfg(feature = "unstable-uniffi")]
uniffi::setup_scaffolding!();

pub mod api;
pub mod authentication;
pub mod canonical_json;
pub mod directory;
pub mod encryption;
pub mod http_headers;
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
pub mod third_party_invite;
pub mod thirdparty;
mod timestamp;
pub mod to_device;

pub mod events;
#[cfg(feature = "html")]
pub mod html;
#[cfg(feature = "olm")]
pub mod olm;
#[cfg(feature = "signatures")]
pub mod signatures;
#[cfg(feature = "state-res")]
pub mod state_res;
#[cfg(feature = "testing")]
pub mod testing;
// Vendored verbatim from `ruma-identifiers-validation`, which documented none
// of these; `harana-matrix-macros` carries a byte-identical copy.
#[allow(missing_docs)]
pub mod validation;

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

/// Alias for [`Sync`] off WASM, an empty trait implemented by everything on it.
///
/// A few futures here borrow a caller-supplied event type across an await, and
/// the SDK built on this crate spawns them, so they have to be `Send`, which in
/// turn makes the borrowed type `Sync`. WASM has no threads and its host types
/// are not `Sync`, so requiring it there would rule out valid callers and
/// nothing else. `sdk_common` has the same pair of markers; this crate
/// cannot use them, since that crate depends on this one.
#[cfg(not(target_family = "wasm"))]
pub trait SyncOutsideWasm: Sync {}
#[cfg(not(target_family = "wasm"))]
impl<T: Sync + ?Sized> SyncOutsideWasm for T {}

/// Alias for [`Sync`] off WASM, an empty trait implemented by everything on it.
#[cfg(target_family = "wasm")]
pub trait SyncOutsideWasm {}
#[cfg(target_family = "wasm")]
impl<T: ?Sized> SyncOutsideWasm for T {}

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
    pub use harana_matrix_macros;
    pub use http;
    pub use serde;
    pub use serde_html_form;
    pub use serde_json;
}
