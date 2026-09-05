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
// See the License for that specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use matrix_sdk::{CallIntentConsensus, EncryptionState, RoomState};
use matrix_sdk_base::RoomInfoNotableUpdateReasons;
use tracing::warn;

use crate::{
    client::JoinRule,
    error::ClientError,
    notification_settings::RoomNotificationMode,
    room::{
        Membership, RoomHero, RoomHistoryVisibility, SuccessorRoom, power_levels::RoomPowerLevels,
    },
    room_member::RoomMember,
    ruma::RtcCallIntent,
};

/// Why a [`RoomInfo`] update was emitted.
///
/// A single update can carry several reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum RoomInfoUpdateReason {
    /// The recency stamp of the room has changed.
    RecencyStamp,
    /// The latest event of the room has changed.
    LatestEvent,
    /// A read receipt has changed.
    ReadReceipt,
    /// The user-controlled unread marker value has changed.
    UnreadMarker,
    /// A membership change happened for the current user.
    Membership,
    /// The display name has changed.
    DisplayName,
    /// The active service members have changed.
    ActiveServiceMembers,
    /// The user's `m.fully_read` marker has changed.
    FullyRead,
    /// A room hero's global profile changed.
    Heroes,
    /// Something else about the room changed.
    ///
    /// The SDK doesn't identify every kind of update yet, so this covers the
    /// rest. Treat it as "re-read whatever you display".
    Unknown,
}

impl RoomInfoUpdateReason {
    /// Split the SDK's bitflags into the reasons they stand for.
    pub(crate) fn from_reasons(reasons: RoomInfoNotableUpdateReasons) -> Vec<Self> {
        // `RoomInfoNotableUpdateReasons::NONE` is the SDK's placeholder for an
        // update it hasn't classified, which is exactly `Unknown` here.
        let mapping = [
            (RoomInfoNotableUpdateReasons::RECENCY_STAMP, Self::RecencyStamp),
            (RoomInfoNotableUpdateReasons::LATEST_EVENT, Self::LatestEvent),
            (RoomInfoNotableUpdateReasons::READ_RECEIPT, Self::ReadReceipt),
            (RoomInfoNotableUpdateReasons::UNREAD_MARKER, Self::UnreadMarker),
            (RoomInfoNotableUpdateReasons::MEMBERSHIP, Self::Membership),
            (RoomInfoNotableUpdateReasons::DISPLAY_NAME, Self::DisplayName),
            (RoomInfoNotableUpdateReasons::ACTIVE_SERVICE_MEMBERS, Self::ActiveServiceMembers),
            (RoomInfoNotableUpdateReasons::FULLY_READ, Self::FullyRead),
            (RoomInfoNotableUpdateReasons::HEROES, Self::Heroes),
            (RoomInfoNotableUpdateReasons::NONE, Self::Unknown),
        ];

        mapping
            .into_iter()
            .filter_map(|(flag, reason)| reasons.contains(flag).then_some(reason))
            .collect()
    }
}

#[derive(Clone, uniffi::Enum)]
pub enum RtcCallIntentConsensus {
    Full(RtcCallIntent),
    Partial { intent: RtcCallIntent, agreeing_count: u64, total_count: u64 },
    None,
}

impl From<CallIntentConsensus> for RtcCallIntentConsensus {
    fn from(value: CallIntentConsensus) -> Self {
        match value {
            CallIntentConsensus::Full(intent) => RtcCallIntentConsensus::Full(intent.into()),
            CallIntentConsensus::Partial { intent, agreeing_count, total_count } => {
                RtcCallIntentConsensus::Partial {
                    intent: intent.into(),
                    agreeing_count,
                    total_count,
                }
            }
            CallIntentConsensus::None => RtcCallIntentConsensus::None,
        }
    }
}

#[derive(uniffi::Record)]
pub struct RoomInfo {
    id: String,
    encryption_state: EncryptionState,
    creators: Option<Vec<String>>,
    /// The room's name from the room state event if received from sync, or one
    /// that's been computed otherwise.
    display_name: Option<String>,
    /// Room name as defined by the room state event only.
    raw_name: Option<String>,
    topic: Option<String>,
    avatar_url: Option<String>,
    is_direct: bool,
    is_dm: bool,
    /// Whether the room is public or not, based on the join rules.
    ///
    /// Can be `None` if the join rules state event is not available for this
    /// room.
    is_public: Option<bool>,
    is_space: bool,
    /// If present, it means the room has been archived/upgraded.
    successor_room: Option<SuccessorRoom>,
    is_favourite: bool,
    is_low_priority: bool,
    canonical_alias: Option<String>,
    alternative_aliases: Vec<String>,
    membership: Membership,
    /// Member who invited the current user to a room that's in the invited
    /// state.
    ///
    /// Can be missing if the room membership invite event is missing from the
    /// store.
    inviter: Option<RoomMember>,
    heroes: Vec<RoomHero>,
    active_members_count: u64,
    invited_members_count: u64,
    joined_members_count: u64,
    active_service_members_count: u64,
    service_members: Vec<String>,
    highlight_count: u64,
    notification_count: u64,
    cached_user_defined_notification_mode: Option<RoomNotificationMode>,
    has_room_call: bool,
    active_room_call_participants: Vec<String>,
    active_room_call_consensus_intent: RtcCallIntentConsensus,
    /// Whether this room has been explicitly marked as unread
    is_marked_unread: bool,
    /// "Interesting" messages received in that room, independently of the
    /// notification settings.
    num_unread_messages: u64,
    /// Events that will notify the user, according to their
    /// notification settings.
    num_unread_notifications: u64,
    /// Events causing mentions/highlights for the user, according to their
    /// notification settings.
    num_unread_mentions: u64,
    /// Event ID of the user's `m.fully_read` marker for this room, if any.
    fully_read_event_id: Option<String>,
    /// The currently pinned event ids.
    pinned_event_ids: Vec<String>,
    /// The join rule for this room, if known.
    join_rule: Option<JoinRule>,
    /// The history visibility for this room, if known.
    history_visibility: RoomHistoryVisibility,
    /// This room's current power levels.
    ///
    /// Can be missing if the room power levels event is missing from the store.
    power_levels: Option<Arc<RoomPowerLevels>>,
    /// This room's version.
    room_version: Option<String>,
    /// Whether creators are privileged over every other user (have infinite
    /// power level).
    privileged_creators_role: bool,
}

impl RoomInfo {
    pub(crate) async fn new(room: &matrix_sdk::Room) -> Result<Self, ClientError> {
        let unread_notification_counts = room.unread_notification_counts();

        let pinned_event_ids =
            room.pinned_event_ids().unwrap_or_default().iter().map(|id| id.to_string()).collect();

        let join_rule = room
            .join_rule()
            .map(TryInto::try_into)
            .transpose()
            .inspect_err(|err| {
                warn!("Failed to parse join rule: {err}");
            })
            .ok()
            .flatten();

        let power_levels = room
            .power_levels()
            .await
            .ok()
            .map(|p| RoomPowerLevels::new(p, room.own_user_id().to_owned()));

        Ok(Self {
            id: room.room_id().to_string(),
            encryption_state: room.encryption_state(),
            creators: room
                .creators()
                .map(|creators| creators.into_iter().map(Into::into).collect()),
            display_name: room.cached_display_name().map(|name| name.to_string()),
            raw_name: room.name(),
            topic: room.topic(),
            avatar_url: room.avatar_url().map(Into::into),
            is_direct: room.is_direct().await?,
            is_dm: room.compute_is_dm().await?,
            is_public: room.is_public(),
            is_space: room.is_space(),
            successor_room: room.successor_room().map(Into::into),
            is_favourite: room.is_favourite(),
            is_low_priority: room.is_low_priority(),
            canonical_alias: room.canonical_alias().map(Into::into),
            alternative_aliases: room.alt_aliases().into_iter().map(Into::into).collect(),
            membership: room.state().into(),
            inviter: match room.state() {
                RoomState::Invited => room
                    .invite_details()
                    .await
                    .ok()
                    .and_then(|details| details.inviter)
                    .map(TryInto::try_into)
                    .transpose()
                    .ok()
                    .flatten(),
                _ => None,
            },
            heroes: room.heroes().await.into_iter().map(Into::into).collect(),
            active_members_count: room.active_members_count(),
            invited_members_count: room.invited_members_count(),
            joined_members_count: room.joined_members_count(),
            active_service_members_count: room.active_service_members_count().unwrap_or_default(),
            service_members: room
                .service_members()
                .iter()
                .flatten()
                .map(|m| m.to_string())
                .collect(),
            highlight_count: unread_notification_counts.highlight_count,
            notification_count: unread_notification_counts.notification_count,
            cached_user_defined_notification_mode: room
                .cached_user_defined_notification_mode()
                .map(Into::into),
            has_room_call: room.has_active_room_call(),
            active_room_call_participants: room
                .active_room_call_participants()
                .iter()
                .map(|u| u.to_string())
                .collect(),
            active_room_call_consensus_intent: room.active_room_call_consensus_intent().into(),
            is_marked_unread: room.is_marked_unread(),
            num_unread_messages: room.num_unread_messages(),
            num_unread_notifications: room.num_unread_notifications(),
            num_unread_mentions: room.num_unread_mentions(),
            fully_read_event_id: room.fully_read_event_id().map(|id| id.to_string()),
            pinned_event_ids,
            join_rule,
            history_visibility: room.history_visibility_or_default().try_into()?,
            power_levels: power_levels.map(Arc::new),
            room_version: room.version().map(|version| version.to_string()),
            privileged_creators_role: room
                .version()
                .and_then(|version| version.rules())
                .map(|rules| rules.authorization.explicitly_privilege_room_creators)
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use matrix_sdk_base::RoomInfoNotableUpdateReasons;

    use super::RoomInfoUpdateReason;

    #[test]
    fn test_a_single_reason_maps_to_its_counterpart() {
        assert_eq!(
            RoomInfoUpdateReason::from_reasons(RoomInfoNotableUpdateReasons::LATEST_EVENT),
            vec![RoomInfoUpdateReason::LatestEvent]
        );

        assert_eq!(
            RoomInfoUpdateReason::from_reasons(RoomInfoNotableUpdateReasons::MEMBERSHIP),
            vec![RoomInfoUpdateReason::Membership]
        );

        // `NONE` is the SDK's placeholder for an update it hasn't classified, which
        // is what `Unknown` means here. It is a reason, not the absence of one.
        assert_eq!(
            RoomInfoUpdateReason::from_reasons(RoomInfoNotableUpdateReasons::NONE),
            vec![RoomInfoUpdateReason::Unknown]
        );
    }

    #[test]
    fn test_several_reasons_are_all_reported() {
        let reasons = RoomInfoUpdateReason::from_reasons(
            RoomInfoNotableUpdateReasons::READ_RECEIPT
                | RoomInfoNotableUpdateReasons::UNREAD_MARKER
                | RoomInfoNotableUpdateReasons::FULLY_READ,
        );

        assert_eq!(
            reasons,
            vec![
                RoomInfoUpdateReason::ReadReceipt,
                RoomInfoUpdateReason::UnreadMarker,
                RoomInfoUpdateReason::FullyRead,
            ]
        );
    }

    #[test]
    fn test_every_reason_is_mapped() {
        // A flag added to the SDK without a counterpart here would silently be
        // dropped from the callback, so check that all of them come through.
        let all = RoomInfoUpdateReason::from_reasons(RoomInfoNotableUpdateReasons::all());

        assert_eq!(all.len(), RoomInfoNotableUpdateReasons::all().iter().count());
    }

    #[test]
    fn test_no_reason_maps_to_nothing() {
        assert!(RoomInfoUpdateReason::from_reasons(RoomInfoNotableUpdateReasons::empty()).is_empty());
    }
}
