//! Common types for the [Matrix](https://matrix.org) protocol.
//!
//! This crate is a vendored and trimmed-down fork of upstream [ruma]'s
//! `ruma-common`. It holds the identifiers, the (de)serialization helpers, the
//! push rules, the canonical JSON support and the core request/response traits of
//! [`api`]; the event types, the endpoint definitions, the HTML sanitizer, the
//! digital signatures and state resolution live in the `events`, `html`,
//! `signatures`, `state-res`, `client-api`, `federation-api` and `appservice-api`
//! crates, all of which the `ruma` facade re-exports under the module names
//! upstream uses.
//!
//! [ruma]: https://github.com/ruma/ruma
//!
//! > For internal consistency, Ruma uses American spelling for variable names. Names may differ
//! > in the serialized representation, as the Matrix specification has a mix of British and
//! > American English.
//!
//! # Cargo features
//!
//! * `client` -- `OutgoingRequest` / `IncomingResponse` impls, for code talking *to* a server.
//! * `server` -- `IncomingRequest` / `OutgoingResponse` impls, for code answering requests.
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

// Lets this crate's own procedural macros resolve the `::common` paths they generate.
extern crate self as common;

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
    pub use http;
    pub use ruma_macros;
    pub use serde;
    pub use serde_html_form;
    pub use serde_json;
}
