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

use client_common::ROOM_VERSION_RULES_FALLBACK;
use common_ruma::{
    OwnedUserId, RoomVersionId, assign,
    events::{
        EmptyStateKey, PossiblyRedactedStateEventContent, RedactContent, RedactedStateEventContent,
        StateEventContent, StateEventType, StaticEventContent,
        macros::EventContent,
        room::create::{PreviousRoom, RoomCreateEventContent},
    },
    room::RoomType,
    room_version_rules::RedactionRules,
};
use serde::{Deserialize, Serialize};

/// The content of an `m.room.create` event, with a required `creator` field.
///
/// Starting with room version 11, the `creator` field should be removed and the
/// `sender` field of the event should be used instead. This is reflected on
/// [`RoomCreateEventContent`].
///
/// This type was created as an alternative for ease of use. When it is used in
/// the SDK, it is constructed by copying the `sender` of the original event as
/// the `creator`.
#[derive(Clone, Debug, Deserialize, Serialize, EventContent)]
#[ruma_event(type = "m.room.create", kind = State, state_key_type = EmptyStateKey, custom_redacted)]
pub struct RoomCreateWithCreatorEventContent {
    /// The `user_id` of the room creator.
    ///
    /// This is set by the homeserver.
    ///
    /// While this should be optional since room version 11, we copy the sender
    /// of the event so we can still access it.
    pub creator: OwnedUserId,

    /// Whether or not this room's data should be transferred to other
    /// homeservers.
    #[serde(
        rename = "m.federate",
        default = "common_ruma::serde::default_true",
        skip_serializing_if = "common_ruma::serde::is_true"
    )]
    pub federate: bool,

    /// The version of the room.
    ///
    /// Defaults to `RoomVersionId::V1`.
    #[serde(default = "default_create_room_version_id")]
    pub room_version: RoomVersionId,

    /// A reference to the room this room replaces, if the previous room was
    /// upgraded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predecessor: Option<PreviousRoom>,

    /// The room type.
    ///
    /// This is currently only used for spaces.
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub room_type: Option<RoomType>,

    /// Additional room creators, considered to have "infinite" power level, in
    /// room versions 12 onwards.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub additional_creators: Vec<OwnedUserId>,
}

impl RoomCreateWithCreatorEventContent {
    /// Constructs a `RoomCreateWithCreatorEventContent` with the given original
    /// content and sender.
    pub fn from_event_content(content: RoomCreateEventContent, sender: OwnedUserId) -> Self {
        let RoomCreateEventContent {
            federate,
            room_version,
            predecessor,
            room_type,
            additional_creators,
            ..
        } = content;
        Self {
            creator: sender,
            federate,
            room_version,
            predecessor,
            room_type,
            additional_creators,
        }
    }

    fn into_event_content(self) -> (RoomCreateEventContent, OwnedUserId) {
        let Self { creator, federate, room_version, predecessor, room_type, additional_creators } =
            self;

        #[allow(deprecated)]
        let content = assign!(RoomCreateEventContent::new_v11(), {
            creator: Some(creator.clone()),
            federate,
            room_version,
            predecessor,
            room_type,
            additional_creators,
        });

        (content, creator)
    }

    /// Get the creators of the room from this content, according to the room
    /// version.
    pub(crate) fn creators(&self) -> Vec<OwnedUserId> {
        let rules = self.room_version.rules().unwrap_or(ROOM_VERSION_RULES_FALLBACK);

        if rules.authorization.explicitly_privilege_room_creators {
            std::iter::once(self.creator.clone())
                .chain(self.additional_creators.iter().cloned())
                .collect()
        } else {
            vec![self.creator.clone()]
        }
    }
}

/// Redacted form of [`RoomCreateWithCreatorEventContent`].
pub type RedactedRoomCreateWithCreatorEventContent = RoomCreateWithCreatorEventContent;

impl RedactedStateEventContent for RedactedRoomCreateWithCreatorEventContent {
    type StateKey = <RoomCreateWithCreatorEventContent as StateEventContent>::StateKey;

    fn event_type(&self) -> StateEventType {
        RoomCreateWithCreatorEventContent::TYPE.into()
    }
}

impl RedactContent for RoomCreateWithCreatorEventContent {
    type Redacted = RedactedRoomCreateWithCreatorEventContent;

    fn redact(self, rules: &RedactionRules) -> Self::Redacted {
        let (content, sender) = self.into_event_content();
        // Use Ruma's redaction algorithm.
        let content = content.redact(rules);
        Self::from_event_content(content, sender)
    }
}

fn default_create_room_version_id() -> RoomVersionId {
    RoomVersionId::V1
}

impl PossiblyRedactedStateEventContent for RoomCreateWithCreatorEventContent {
    type StateKey = <RoomCreateWithCreatorEventContent as StateEventContent>::StateKey;

    fn event_type(&self) -> StateEventType {
        RoomCreateWithCreatorEventContent::TYPE.into()
    }
}

#[cfg(test)]
mod tests {
    use assert_matches2::assert_let;
    use common_ruma::{
        RoomVersionId,
        events::{RedactContent, room::create::RoomCreateEventContent},
        owned_room_id, owned_user_id,
        room::RoomType,
        room_version_rules::RedactionRules,
    };
    use serde_json::json;

    use super::RoomCreateWithCreatorEventContent;

    /// Deserializing an `m.room.create` content must apply the defaults
    /// mandated by the spec: `room_version` defaults to `1`, `m.federate`
    /// defaults to `true`, and every other field is optional.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mroomcreate>.
    #[test]
    fn test_deserialization_applies_the_spec_defaults() {
        let content: RoomCreateWithCreatorEventContent =
            serde_json::from_value(json!({ "creator": "@alice:localhost" })).unwrap();

        assert_eq!(content.creator, owned_user_id!("@alice:localhost"));
        assert_eq!(content.room_version, RoomVersionId::V1);
        assert!(content.federate);
        assert!(content.predecessor.is_none());
        assert!(content.room_type.is_none());
        assert!(content.additional_creators.is_empty());
    }

    /// `m.federate` set to `false` must be round-tripped, and the default
    /// (`true`) must not be serialized.
    #[test]
    fn test_federate_is_round_tripped() {
        let content: RoomCreateWithCreatorEventContent = serde_json::from_value(json!({
            "creator": "@alice:localhost",
            "m.federate": false,
        }))
        .unwrap();

        assert!(!content.federate);
        assert_eq!(
            serde_json::to_value(&content).unwrap(),
            json!({
                "creator": "@alice:localhost",
                "m.federate": false,
                "room_version": "1",
            })
        );

        let content: RoomCreateWithCreatorEventContent = serde_json::from_value(json!({
            "creator": "@alice:localhost",
            "room_version": "11",
        }))
        .unwrap();

        assert!(content.federate);
        // `m.federate: true` is the default, so it is skipped.
        assert_eq!(
            serde_json::to_value(&content).unwrap(),
            json!({
                "creator": "@alice:localhost",
                "room_version": "11",
            })
        );
    }

    /// A room created by upgrading another one carries a `predecessor`.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#room-upgrades>.
    #[test]
    fn test_predecessor_is_deserialized() {
        let content: RoomCreateWithCreatorEventContent = serde_json::from_value(json!({
            "creator": "@alice:localhost",
            "room_version": "11",
            "predecessor": {
                "room_id": "!old:localhost",
                "event_id": "$tombstone",
            },
        }))
        .unwrap();

        assert_let!(Some(predecessor) = &content.predecessor);
        assert_eq!(predecessor.room_id, owned_room_id!("!old:localhost"));
    }

    /// A space is an `m.room.create` event with a `type` of `m.space`.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mspace>.
    #[test]
    fn test_space_room_type_is_deserialized() {
        let content: RoomCreateWithCreatorEventContent = serde_json::from_value(json!({
            "creator": "@alice:localhost",
            "room_version": "11",
            "type": "m.space",
        }))
        .unwrap();

        assert_eq!(content.room_type, Some(RoomType::Space));
    }

    /// Starting with room version 11 the `creator` field is gone from the
    /// content, and the `sender` of the event must be used instead.
    ///
    /// See <https://spec.matrix.org/v1.16/rooms/v11/#mroomcreate-schema>.
    #[test]
    fn test_creator_is_taken_from_the_sender() {
        let sender = owned_user_id!("@alice:localhost");
        let content = RoomCreateWithCreatorEventContent::from_event_content(
            RoomCreateEventContent::new_v11(),
            sender.clone(),
        );

        assert_eq!(content.creator, sender);
        assert_eq!(content.room_version, RoomVersionId::V11);
    }

    /// Before room version 12, only the single `creator` is privileged;
    /// `additional_creators` must be ignored.
    #[test]
    fn test_creators_before_room_version_12() {
        let content = RoomCreateWithCreatorEventContent {
            creator: owned_user_id!("@alice:localhost"),
            federate: true,
            room_version: RoomVersionId::V11,
            predecessor: None,
            room_type: None,
            additional_creators: vec![owned_user_id!("@bob:localhost")],
        };

        assert_eq!(content.creators(), vec![owned_user_id!("@alice:localhost")]);
    }

    /// From room version 12, the creator and the `additional_creators` are all
    /// privileged.
    ///
    /// See <https://spec.matrix.org/v1.16/rooms/v12/#creators>.
    #[test]
    fn test_creators_from_room_version_12() {
        let content = RoomCreateWithCreatorEventContent {
            creator: owned_user_id!("@alice:localhost"),
            federate: true,
            room_version: RoomVersionId::V12,
            predecessor: None,
            room_type: None,
            additional_creators: vec![
                owned_user_id!("@bob:localhost"),
                owned_user_id!("@carol:localhost"),
            ],
        };

        assert_eq!(
            content.creators(),
            vec![
                owned_user_id!("@alice:localhost"),
                owned_user_id!("@bob:localhost"),
                owned_user_id!("@carol:localhost"),
            ]
        );
    }

    /// An unknown room version has no rules; we fall back to the room version
    /// 11 rules, where only the single creator is privileged.
    #[test]
    fn test_creators_with_an_unknown_room_version() {
        let content = RoomCreateWithCreatorEventContent {
            creator: owned_user_id!("@alice:localhost"),
            federate: true,
            room_version: "org.example.unknown".parse().unwrap(),
            predecessor: None,
            room_type: None,
            additional_creators: vec![owned_user_id!("@bob:localhost")],
        };

        assert_eq!(content.creators(), vec![owned_user_id!("@alice:localhost")]);
    }

    /// Before room version 11, redacting an `m.room.create` event keeps only
    /// the `creator` key of its content.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#redactions>.
    #[test]
    fn test_redaction_before_room_version_11_keeps_only_the_creator() {
        let content = RoomCreateWithCreatorEventContent {
            creator: owned_user_id!("@alice:localhost"),
            federate: false,
            room_version: RoomVersionId::V10,
            predecessor: None,
            room_type: Some(RoomType::Space),
            additional_creators: vec![owned_user_id!("@bob:localhost")],
        };

        let redacted = content.redact(&RedactionRules::V1);

        // The creator survives the redaction…
        assert_eq!(redacted.creator, owned_user_id!("@alice:localhost"));
        // …everything else is reset to its default.
        assert!(redacted.federate);
        assert_eq!(redacted.room_version, RoomVersionId::V1);
        assert!(redacted.room_type.is_none());
        assert!(redacted.additional_creators.is_empty());
    }

    /// From room version 11, the whole content of an `m.room.create` event is
    /// kept when it is redacted.
    ///
    /// See <https://spec.matrix.org/v1.16/rooms/v11/#redactions>.
    #[test]
    fn test_redaction_from_room_version_11_keeps_the_whole_content() {
        let content = RoomCreateWithCreatorEventContent {
            creator: owned_user_id!("@alice:localhost"),
            federate: false,
            room_version: RoomVersionId::V11,
            predecessor: None,
            room_type: Some(RoomType::Space),
            additional_creators: vec![owned_user_id!("@bob:localhost")],
        };

        let redacted = content.clone().redact(&RedactionRules::V11);

        assert_eq!(redacted.creator, content.creator);
        assert_eq!(redacted.federate, content.federate);
        assert_eq!(redacted.room_version, content.room_version);
        assert_eq!(redacted.room_type, content.room_type);
        assert_eq!(redacted.additional_creators, content.additional_creators);
    }
}
