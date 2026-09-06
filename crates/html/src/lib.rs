//! Opinionated HTML parsing and manipulating library.
//!
//! Like the rest of the Ruma crates, this crate is primarily meant to be used for
//! the Matrix protocol. It should be able to be used to interact with any HTML
//! document but will offer APIs focused on specificities of HTML in the Matrix
//! specification..
//!

#![warn(missing_docs)]

#![recursion_limit = "1024"]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(trivial_casts)]
#![allow(clippy::new_without_default)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use html5ever::{Attribute, LocalName, Namespace, Prefix, QualName, tendril::StrTendril};

mod helpers;
mod dom;
mod sanitizer_config;

pub use self::{helpers::*, dom::*, sanitizer_config::*};

/// What [HTML elements and attributes] should be kept by the sanitizer.
///
/// [HTML elements and attributes]: https://spec.matrix.org/v1.19/client-server-api/#mroommessage-msgtypes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum HtmlSanitizerMode {
    /// Keep only the elements and attributes suggested in the Matrix specification.
    ///
    /// In addition to filtering elements and attributes listed in the Matrix specification, it
    /// also removes elements that are nested more than 100 levels deep.
    ///
    /// Deprecated elements and attributes are also replaced when applicable.
    Strict,

    /// Like `Strict` mode, with additional elements and attributes that are not yet included in
    /// the spec, but are reasonable to keep.
    ///
    /// Differences with `Strict` mode:
    ///
    /// * The `matrix` scheme is allowed in links.
    Compat,
}

/// The paths this crate used when it was a module of `ruma`.
///
/// Not part of the public API.
#[doc(hidden)]
pub mod __ruma {
    pub use common::*;

    pub use crate as html;
}
