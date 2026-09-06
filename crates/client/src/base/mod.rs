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

#![doc = include_str!("../../docs/base.md")]
#![cfg_attr(target_family = "wasm", allow(clippy::arc_with_non_send_sync))]
#![warn(missing_docs, missing_debug_implementations)]
// Async methods must hand back futures that can be spawned on a
// multi-threaded runtime, which is what consumers of this crate do with
// them. WASM has no threads and its host types are not `Send`, so the lint
// is only applied elsewhere.
#![cfg_attr(not(target_family = "wasm"), deny(clippy::future_not_send))]

use harana_matrix_common::{OwnedDeviceId, OwnedUserId};
use serde::{Deserialize, Serialize};

pub use crate::{
    base::error::{Error, Result},
    common::*,
};

mod client;
pub use client::RequestedRequiredStates;
pub mod debug;
pub mod deserialized_responses;
mod error;
pub mod event_cache;
pub mod latest_event;
pub mod media;
pub mod notification_settings;
pub mod read_receipts;
mod response_processors;
mod room;

pub mod sliding_sync;

pub mod store;
pub mod sync;
#[cfg(any(test, feature = "testing"))]
mod test_utils;
pub mod to_device_token;
mod utils;

pub use client::DmRoomDefinition;

#[cfg(feature = "experimental-element-recent-emojis")]
pub mod recent_emojis;

pub use client::{BaseClient, ThreadingSupport};
#[cfg(any(test, feature = "testing"))]
pub use http;
pub use room::{
    CallIntentConsensus, EncryptionState, MembersRequestGuard, PredecessorRoom, Room,
    RoomCreateWithCreatorEventContent, RoomDisplayName, RoomHero, RoomHeroWithProfile, RoomInfo,
    RoomInfoNotableUpdate, RoomInfoNotableUpdateReasons, RoomMember, RoomMembersUpdate,
    RoomMemberships, RoomRecencyStamp, RoomState, RoomStateFilter, SuccessorRoom, apply_redaction,
};
pub use store::{
    ComposerDraft, ComposerDraftType, DraftAttachment, DraftAttachmentContent, DraftThumbnail,
    QueueWedgeError, StateChanges, StateStore, StateStoreDataKey, StateStoreDataValue, StoreError,
    ThreadSubscriptionCatchupToken,
};
pub use utils::{MinimalRoomMemberEvent, MinimalStateEvent, RawStateEventWithKeys};

#[cfg(feature = "e2e-encryption")]
pub use crate::crypto;

/// The Matrix user session info.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SessionMeta {
    /// The ID of the session's user.
    pub user_id: OwnedUserId,
    /// The ID of the client device.
    pub device_id: OwnedDeviceId,
}

// `#[macro_export]` puts these at the crate root; re-export them here too, so
// that they are reachable under the module that defines them, as they were when
// it was its own crate.
#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub use crate::{
    event_cache_store_integration_tests, event_cache_store_integration_tests_time,
    media_store_inner_integration_tests, media_store_integration_tests,
    media_store_integration_tests_time, statestore_integration_tests,
};
