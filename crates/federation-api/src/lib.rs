//! (De)serializable types for the [Matrix Server-Server API][federation-api].
//! These types are used by server code.
//!
//! [federation-api]: https://spec.matrix.org/v1.19/server-server-api/

// This crate is not useful without either of those features, so export nothing if they are not
// enabled to avoid errors when running checks wrongly without enabling any of them.
#![warn(missing_docs)]

#![recursion_limit = "1024"]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(trivial_casts)]
#![allow(clippy::new_without_default)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod serde;

pub mod authenticated_media;
pub mod authentication;
pub mod authorization;
pub mod backfill;
pub mod device;
pub mod directory;
pub mod discovery;
pub mod event;
pub mod keys;
pub mod membership;
pub mod openid;
pub mod policy;
pub mod query;
pub mod room;
pub mod space;
pub mod thirdparty;
pub mod transactions;

#[doc(hidden)]
pub use crate::__ruma::PrivOwnedStr;

/// The paths this crate used when it was a module of `ruma`.
///
/// Not part of the public API.
#[doc(hidden)]
pub mod __ruma {
    pub use common::*;

    pub use ::events;
    pub use ::signatures;

    pub mod api {
        pub use common::api::*;

        pub use crate as federation;
    }
}
