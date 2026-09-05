// Copyright 2025 The Matrix.org Foundation C.I.C.
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

//! The Matrix types the SDK's API is written in terms of.
//!
//! These come from [`ruma`], which models the Matrix specification. They are
//! re-exported here so that using the SDK doesn't require adding a direct
//! dependency on `ruma` and matching its version: `matrix_sdk::ruma` (or
//! [`crate::ruma`]) gives access to all of it, and this module collects the
//! handful of types that turn up in nearly every SDK signature.
//!
//! # Identifiers
//!
//! Matrix identifiers come in pairs: a borrowed, unsized type such as
//! [`UserId`], and its owned counterpart [`OwnedUserId`], in the same way as
//! [`str`] and [`String`]. Parse one from a string with `UserId::parse`, or
//! build one at compile time with the `user_id!` macro from
//! [`ruma`][crate::ruma].
//!
//! ```
//! use matrix_sdk_common::{ruma::user_id, types::UserId};
//!
//! // Checked at compile time.
//! let alice = user_id!("@alice:example.org");
//!
//! // Checked at run time.
//! let bob = UserId::parse("@bob:example.org").unwrap();
//!
//! assert_eq!(alice.server_name(), bob.server_name());
//! ```
//!
//! # Events
//!
//! An event that the SDK hands out is usually wrapped in a [`Raw`], which holds
//! the original JSON. Call [`Raw::deserialize()`] to get the typed event, or
//! [`Raw::deserialize_as`] to read it as a different type: keeping the JSON
//! means an event the SDK doesn't know about is still available to the caller,
//! and that redactions and signatures can be checked against exactly what the
//! server sent.

pub use ruma::{
    DeviceId, EventId, MilliSecondsSinceUnixEpoch, MxcUri, OwnedDeviceId, OwnedEventId, OwnedMxcUri,
    OwnedRoomAliasId, OwnedRoomId, OwnedServerName, OwnedTransactionId, OwnedUserId, RoomAliasId,
    RoomId, RoomVersionId, SecondsSinceUnixEpoch, ServerName, TransactionId, UInt, UserId,
    events::{
        AnyMessageLikeEventContent, AnyStateEventContent, AnySyncMessageLikeEvent,
        AnySyncStateEvent, AnySyncTimelineEvent, AnyTimelineEvent, MessageLikeEventType,
        StateEventType, TimelineEventType,
    },
    serde::Raw,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruma::{events::AnySyncTimelineEvent as RumaAnySyncTimelineEvent, user_id};

    /// The re-exports are the `ruma` types themselves, not copies of them, so a
    /// value built through `ruma` can be used wherever these name a type.
    #[test]
    fn test_the_types_are_rumas() {
        fn server_name_of(user_id: &UserId) -> &ServerName {
            user_id.server_name()
        }

        // Built by `ruma`'s macro, accepted by a function written in terms of the
        // re-export.
        assert_eq!(server_name_of(user_id!("@alice:example.org")), "example.org");

        let parsed: OwnedUserId = UserId::parse("@alice:example.org").unwrap();
        assert_eq!(server_name_of(&parsed), "example.org");

        // Same for the event types: a `Raw` named through this module deserializes
        // into `ruma`'s event enum.
        let raw: Raw<AnySyncTimelineEvent> = Raw::from_json_string(
            serde_json::json!({
                "type": "m.room.message",
                "event_id": "$1",
                "sender": "@alice:example.org",
                "origin_server_ts": 42,
                "content": { "msgtype": "m.text", "body": "hello" },
            })
            .to_string(),
        )
        .unwrap();

        let event: RumaAnySyncTimelineEvent = raw.deserialize().unwrap();

        assert_eq!(event.event_id(), "$1");
        assert_eq!(event.sender(), parsed);
    }
}
