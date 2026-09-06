// Copyright 2020 Damir Jelić
// Copyright 2020 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![recursion_limit = "256"]
#![doc = include_str!("../README.md")]
//!
//! # Matrix types
//!
//! The SDK's API is written in terms of the Matrix types from [`ruma`], which
//! is re-exported here as [`client_matrix::ruma`][ruma] so that using them doesn't
//! require a direct dependency on it. The ones that turn up in nearly every
//! signature, such as [`OwnedUserId`][types::OwnedUserId] and
//! [`Raw`][types::Raw], are collected in [`client_matrix::types`][types], which is
//! the place to start.
//!
//! # Matrix versions
//!
//! The types and endpoints this SDK is built on model the [Matrix
//! specification] up to **v1.19**, the newest value of
//! [`MatrixVersion`][common_ruma::api::MatrixVersion]. That is the ceiling: it says
//! what the SDK knows how to speak, not what any given homeserver answers to.
//!
//! ## What the SDK does with the server's answer
//!
//! A [`Client`] asks the homeserver for its `/_matrix/client/versions` at
//! startup and keeps the answer, so it knows both the Matrix versions the
//! server implements and the unstable feature flags it advertises. That answer
//! is persisted in the store alongside the time it was fetched, refreshed once
//! older than [`ClientBuilder::discovery_cache_timeout`], and can be refreshed
//! on demand with [`Client::rediscover`].
//!
//! The SDK then adapts to it, rather than assuming a version:
//!
//! * **Endpoint versions.** Every request carries the history of the paths it
//!   has lived at. The path is chosen per request from what the server
//!   advertises: the stable path when the server implements the Matrix version
//!   that stabilised it, the unstable path when it only advertises the feature
//!   flag the endpoint shipped behind, and an error when it offers neither.
//!   Nothing has to be configured for this.
//! * **Picking between two ways of doing one thing.** Where an older and a
//!   newer endpoint both exist and the choice changes behaviour rather than
//!   just the URL (authenticated media, deleting a profile field), the SDK asks
//!   whether the newer one is supported and falls back if it isn't.
//!   [`Client::supports_endpoint`] is that question, and is available to
//!   callers making the same kind of choice in their own code.
//! * **Room versions.** Rules that differ per room version, such as where a
//!   redaction names the event it redacts, are read off the room's own version
//!   rather than the server's.
//!
//! ## Inspecting and steering it
//!
//! * [`Client::supported_versions`] returns the versions and feature flags
//!   together, [`Client::server_versions`] and [`Client::unstable_features`]
//!   one each.
//! * [`Client::supports_endpoint`] answers the question for one endpoint.
//! * [`ClientBuilder::server_versions`] supplies the answer up front for a
//!   server whose versions are already known, which skips the request.
//! * [`Client::rediscover`] re-asks, for a server that has been upgraded
//!   underneath a running client.
//!
//! Note that a homeserver advertising a Matrix version is a claim to implement
//! all of it, so an endpoint that names neither a stable version nor a feature
//! flag cannot be detected: [`Client::supports_endpoint`] answers `false` for
//! it, and the only way to find out is to send the request.
//!
//! [Matrix specification]: https://spec.matrix.org/v1.19/
//! [`Client`]: crate::Client
//! [`ClientBuilder::discovery_cache_timeout`]: crate::ClientBuilder::discovery_cache_timeout
//! [`ClientBuilder::server_versions`]: crate::ClientBuilder::server_versions
#![warn(missing_debug_implementations, missing_docs)]
// Async methods must hand back futures that can be spawned on a
// multi-threaded runtime, which is what consumers of this crate do with
// them. WASM has no threads and its host types are not `Send`, so the lint
// is only applied elsewhere.
#![cfg_attr(not(target_family = "wasm"), deny(clippy::future_not_send))]
#![cfg_attr(target_family = "wasm", allow(clippy::arc_with_non_send_sync))]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use async_trait::async_trait;
pub use bytes;
pub use client_base::{
    CallIntentConsensus, ComposerDraft, ComposerDraftType, DraftAttachment, DraftAttachmentContent,
    DraftThumbnail, EncryptionState, PredecessorRoom, QueueWedgeError, Room as BaseRoom,
    RoomCreateWithCreatorEventContent, RoomDisplayName, RoomHero, RoomHeroWithProfile, RoomInfo,
    RoomMember as BaseRoomMember, RoomMembersUpdate, RoomMemberships, RoomRecencyStamp, RoomState,
    SessionMeta, StateChanges, StateStore, StoreError, SuccessorRoom, ThreadingSupport,
    deserialized_responses,
    store::{self, DynStateStore, MemoryStore, StateStoreExt},
};
pub use client_common::*;
#[cfg(feature = "reqwest-transport")]
pub use reqwest;

mod account;
pub mod attachment;
pub mod authentication;
mod client;
pub mod config;
mod deduplicating_handler;
#[cfg(feature = "e2e-encryption")]
pub mod encryption;
mod error;
pub mod event_cache;
pub mod event_handler;
mod http_client;
pub mod latest_events;
pub mod media;
pub mod notification_settings;
pub mod paginators;
pub mod pusher;
pub mod room;
pub mod room_directory_search;
pub mod room_preview;
pub mod send_queue;
pub mod utils;
pub mod futures {
    //! Named futures returned from methods on types in [the crate root][crate].

    pub use super::client::futures::SendRequest;
}
pub mod sliding_sync;
pub mod sync;
#[cfg(feature = "experimental-widgets")]
pub mod widget;

#[cfg(feature = "experimental-search-core")]
pub mod message_search;

pub use account::Account;
pub use authentication::{AuthApi, AuthSession, SessionTokens};
pub use client::homeserver_capabilities::HomeserverCapabilities;
#[cfg(feature = "experimental-search-core")]
pub mod search_index;
pub use client::{
    Client, ClientBuildError, ClientBuilder, LoopCtrl, ServerVendorInfo, SessionChange,
    StoreProvider, StoreProviderError, StoreSizes, TileServerInfo, sanitize_server_name,
};
pub use error::{
    BeaconError, Error, HttpError, HttpResult, NotificationSettingsError, RefreshTokenError,
    Result, RumaApiError,
};
#[cfg(feature = "reqwest-transport")]
pub use http_client::ReqwestTransport;
pub use http_client::{
    HttpSend, RequestProgress, SupportedAuthScheme, SupportedPathBuilder, TransmissionProgress,
};
#[cfg(all(feature = "e2e-encryption", feature = "sqlite"))]
pub use client_sqlite::SqliteCryptoStore;
#[cfg(feature = "sqlite")]
pub use client_sqlite::log_targets as sqlite_log_targets;
#[cfg(feature = "sqlite")]
pub use client_sqlite::pluggable as store_encryption;
#[cfg(feature = "sqlite")]
pub use client_sqlite::{
    STATE_STORE_DATABASE_NAME, SecretStoreCipherProvider, SqliteEventCacheStore, SqliteMediaStore,
    SqliteStateStore, SqliteStoreConfig,
};
pub use media::Media;
pub use pusher::Pusher;
pub use room::Room;
pub use common_ruma::{IdParseError, OwnedServerName, ServerName};
pub use sliding_sync::{
    SlidingSync, SlidingSyncBuilder, SlidingSyncList, SlidingSyncListBuilder,
    SlidingSyncListLoadingState, SlidingSyncMode, UpdateSummary,
};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod live_locations_observer;

#[cfg(feature = "unstable-msc4426")]
mod automatic_call_status;

#[cfg(any(test, feature = "testing"))]
pub mod test_utils;

#[cfg(test)]
common_test_utils::init_tracing_for_tests!();
