//! Building blocks for a Matrix homeserver.
//!
//! Each module is an independent piece of server-side machinery, behind a
//! feature of the same name (all on by default):
//!
//! | module         | feature        | what it does                                                     |
//! | -------------- | -------------- | ---------------------------------------------------------------- |
//! | [`appservice`] | `appservice`   | application service registration files and namespace matching     |
//! | [`resolver`]   | `resolver`     | the `.well-known` and SRV ladder of the server-server API         |
//! | [`state_res`]  | `state-res`    | store-backed adapters over state resolution and event auth        |
//! | [`store_codec`]| `store-codec`  | an order-preserving binary codec for key-value store records      |
//! | [`thumbnail`]  | `thumbnail`    | thumbnail generation with a bounded decode budget                 |
//!
//! The protocol types they are built on live in [`harana_matrix_common`].

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "appservice")]
pub mod appservice;
#[cfg(feature = "resolver")]
pub mod resolver;
#[cfg(feature = "state-res")]
pub mod state_res;
#[cfg(feature = "store-codec")]
pub mod store_codec;
#[cfg(feature = "thumbnail")]
pub mod thumbnail;
