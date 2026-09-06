#![doc = include_str!("../README.md")]
#![forbid(missing_docs)]

/// Monotonically increasing timestamp of operations on the index.
pub type OpStamp = u64;

#[cfg(feature = "tantivy")]
pub(crate) const TANTIVY_INDEX_MEMORY_BUDGET: usize = 50_000_000;

#[cfg(feature = "tantivy")]
mod encrypted;
#[cfg(feature = "tantivy")]
mod schema;
#[cfg(feature = "tantivy")]
mod writer;

pub mod backend;
/// A module for errors relating to the search crate.
pub mod error;
/// A module for the built-in Tantivy search index.
#[cfg(feature = "tantivy")]
pub mod index;
