#![doc = include_str!("../README.md")]
#![forbid(missing_docs)]

#[macro_use]
mod macros;

pub mod endpoint;
mod error;
mod extract;
mod handler;
mod response;
mod router;
pub mod routes;

pub use self::{
    endpoint::{Api, AuthKind, EndpointMeta},
    error::{Error, Result},
    extract::Ruma,
    handler::RumaHandler,
    response::RumaResponse,
    router::MatrixRouter,
};
