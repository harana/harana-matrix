//! The endpoints of every Matrix API a homeserver serves.
//!
//! Each submodule declares the endpoints of one API, taken from the modules of
//! [`ruma::api`]. They are what
//! [`MatrixRouter::new()`](crate::MatrixRouter::new) registers, and [`all()`]
//! describes them.

pub mod client;
pub mod federation;

use crate::{EndpointMeta, MatrixRouter};

/// The description of every endpoint this crate knows about.
///
/// Which endpoints those are depends on the features this crate was built with:
/// the endpoints of an unstable MSC are only there if its feature is enabled.
pub fn all() -> Vec<EndpointMeta> {
    let mut endpoints = Vec::new();

    client::describe(&mut endpoints);
    federation::describe(&mut endpoints);

    endpoints
}

/// Register a stub route for every endpoint this crate knows about.
pub(crate) fn register_all<S>(router: &mut MatrixRouter<S>)
where
    S: Clone + Send + Sync + 'static,
{
    client::register_stubs(router);
    federation::register_stubs(router);
}
