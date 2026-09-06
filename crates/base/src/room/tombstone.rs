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

use std::collections::BTreeSet;

use ruma::{
    OwnedRoomId, OwnedServerName, UserId,
    events::room::tombstone::PossiblyRedactedRoomTombstoneEventContent,
};

use super::Room;

/// How many candidate servers at most to put in a `via` list.
///
/// A `via` list is a hint, not an exhaustive list of the servers in a room, and
/// every entry costs the resolving server a request.
const MAX_VIA_SERVERS: usize = 5;

impl Room {
    /// Has the room been tombstoned.
    ///
    /// A room is tombstoned if it has received a [`m.room.tombstone`] state
    /// event; see [`Room::tombstone_content`].
    ///
    /// [`m.room.tombstone`]: https://spec.matrix.org/v1.14/client-server-api/#mroomtombstone
    pub fn is_tombstoned(&self) -> bool {
        self.info.read().is_tombstoned()
    }

    /// Get the [`m.room.tombstone`] state event's content of this room if one
    /// has been received.
    ///
    /// Also see [`Room::is_tombstoned`] to check if the [`m.room.tombstone`]
    /// event has been received. It's faster than using this method.
    ///
    /// [`m.room.tombstone`]: https://spec.matrix.org/v1.14/client-server-api/#mroomtombstone
    pub fn tombstone_content(&self) -> Option<PossiblyRedactedRoomTombstoneEventContent> {
        self.info.read().tombstone().cloned()
    }

    /// If this room is tombstoned, return the “reference” to the successor room
    /// —i.e. the room replacing this one.
    ///
    /// A room is tombstoned if it has received a [`m.room.tombstone`] state
    /// event; see [`Room::tombstone_content`].
    ///
    /// [`m.room.tombstone`]: https://spec.matrix.org/v1.14/client-server-api/#mroomtombstone
    pub fn successor_room(&self) -> Option<SuccessorRoom> {
        let info = self.info.read();
        let tombstone_event = info.tombstone()?.clone();
        let room_id = tombstone_event.replacement_room?;

        // The server of whoever tombstoned the room certainly knows the successor
        // room, since it is the server that created it. The other members of this
        // room are the ones most likely to have followed the tombstone, so their
        // servers come next.
        let via = via_servers(
            info.base_info
                .tombstone_sender
                .as_deref()
                .into_iter()
                .chain(info.heroes().iter().map(|hero| hero.user_id.as_ref())),
        );

        Some(SuccessorRoom {
            room_id,
            reason: tombstone_event.body.filter(|body| !body.is_empty()),
            via,
        })
    }

    /// If this room is the successor of a tombstoned room, return the
    /// “reference” to the predecessor room.
    ///
    /// A room is tombstoned if it has received a [`m.room.tombstone`] state
    /// event; see [`Room::tombstone_content`].
    ///
    /// To determine if a room is the successor of a tombstoned room, the
    /// [`m.room.create`] must have been received, **with** a `predecessor`
    /// field. See [`Room::create_content`].
    ///
    /// [`m.room.tombstone`]: https://spec.matrix.org/v1.14/client-server-api/#mroomtombstone
    /// [`m.room.create`]: https://spec.matrix.org/v1.14/client-server-api/#mroomcreate
    pub fn predecessor_room(&self) -> Option<PredecessorRoom> {
        let info = self.info.read();
        let create_content = info.create()?.clone();
        let room_id = create_content.predecessor?.room_id;

        // Whoever created this room was in the predecessor room to upgrade it, and so
        // were we, so both our servers know it. The other members of this room are
        // likely to have been in it too.
        let via = via_servers(
            [create_content.creator.as_ref(), self.own_user_id.as_ref()]
                .into_iter()
                .chain(info.heroes().iter().map(|hero| hero.user_id.as_ref())),
        );

        Some(PredecessorRoom { room_id, via })
    }
}

/// Collect the servers of the given users, in order, without duplicates and
/// capped at [`MAX_VIA_SERVERS`].
fn via_servers<'a>(users: impl Iterator<Item = &'a UserId>) -> Vec<OwnedServerName> {
    let mut seen = BTreeSet::new();
    let mut servers = Vec::new();

    for user_id in users {
        let server_name = user_id.server_name();

        if seen.insert(server_name) {
            servers.push(server_name.to_owned());

            if servers.len() == MAX_VIA_SERVERS {
                break;
            }
        }
    }

    servers
}

/// When a room A is tombstoned, it is replaced by a room B. The room A is the
/// predecessor of B, and B is the successor of A. This type holds information
/// about the successor room. See [`Room::successor_room`].
///
/// A room is tombstoned if it has received a [`m.room.tombstone`] state event.
///
/// [`m.room.tombstone`]: https://spec.matrix.org/v1.14/client-server-api/#mroomtombstone
#[derive(Debug)]
pub struct SuccessorRoom {
    /// The ID of the next room replacing this (tombstoned) room.
    pub room_id: OwnedRoomId,

    /// The reason why the room has been tombstoned.
    pub reason: Option<String>,

    /// Candidate servers to pass as `via` when previewing or joining the
    /// successor room.
    ///
    /// A room ID is not routable on its own: a server that has never seen the
    /// successor room cannot resolve it, which is what stops a user on another
    /// server from following the tombstone. These are the servers most likely
    /// to know the room: the one of whoever tombstoned this room, which created
    /// the successor, followed by those of this room's heroes, who are the
    /// members most likely to have followed the tombstone.
    ///
    /// This is a best-effort hint and may be empty.
    pub via: Vec<OwnedServerName>,
}

/// When a room A is tombstoned, it is replaced by a room B. The room A is the
/// predecessor of B, and B is the successor of A. This type holds information
/// about the predecessor room. See [`Room::predecessor_room`].
///
/// To know the predecessor of a room, the [`m.room.create`] state event must
/// have been received.
///
/// [`m.room.create`]: https://spec.matrix.org/v1.14/client-server-api/#mroomcreate
#[derive(Debug)]
pub struct PredecessorRoom {
    /// The ID of the old room.
    pub room_id: OwnedRoomId,

    /// Candidate servers to pass as `via` when previewing or joining the
    /// predecessor room.
    ///
    /// See [`SuccessorRoom::via`]. Here they are the servers of the creator of
    /// this room, of our own user, and of this room's heroes: all of them were
    /// plausibly in the predecessor room.
    ///
    /// This is a best-effort hint and may be empty.
    pub via: Vec<OwnedServerName>,
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use assert_matches::assert_matches;
    use ruma::{RoomVersionId, room_id, server_name, user_id};
    use sdk_test::{
        JoinedRoomBuilder, SyncResponseBuilder, async_test, event_factory::EventFactory,
    };

    use crate::{RoomState, test_utils::logged_in_base_client};

    #[async_test]
    async fn test_no_successor_room() {
        let client = logged_in_base_client(None).await;
        let room = client.get_or_create_room(room_id!("!r0"), RoomState::Joined);

        assert!(room.is_tombstoned().not());
        assert!(room.tombstone_content().is_none());
        assert!(room.successor_room().is_none());
    }

    #[async_test]
    async fn test_successor_room() {
        let client = logged_in_base_client(None).await;
        let sender = user_id!("@mnt_io:matrix.org");
        let room_id = room_id!("!r0");
        let successor_room_id = room_id!("!r1");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        let mut sync_builder = SyncResponseBuilder::new();
        let response = sync_builder
            .add_joined_room(
                JoinedRoomBuilder::new(room_id).add_timeline_event(
                    EventFactory::new()
                        .sender(sender)
                        .room_tombstone("traces of you", successor_room_id),
                ),
            )
            .build_sync_response();

        client.receive_sync_response(response).await.unwrap();

        assert!(room.is_tombstoned());
        assert!(room.tombstone_content().is_some());
        assert_matches!(room.successor_room(), Some(successor_room) => {
            assert_eq!(successor_room.room_id, successor_room_id);
            assert_matches!(successor_room.reason, Some(reason) => {
                assert_eq!(reason, "traces of you");
            });
            // The server of whoever tombstoned the room is the one that knows the
            // successor room.
            assert_eq!(successor_room.via, vec![server_name!("matrix.org").to_owned()]);
        });
    }

    #[async_test]
    async fn test_successor_room_no_reason() {
        let client = logged_in_base_client(None).await;
        let sender = user_id!("@mnt_io:matrix.org");
        let room_id = room_id!("!r0");
        let successor_room_id = room_id!("!r1");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        let mut sync_builder = SyncResponseBuilder::new();
        let response = sync_builder
            .add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_event(
                EventFactory::new().sender(sender).room_tombstone(
                    // An empty reason will result in `None` in `SuccessorRoom::reason`.
                    "",
                    successor_room_id,
                ),
            ))
            .build_sync_response();

        client.receive_sync_response(response).await.unwrap();

        assert!(room.is_tombstoned());
        assert!(room.tombstone_content().is_some());
        assert_matches!(room.successor_room(), Some(successor_room) => {
            assert_eq!(successor_room.room_id, successor_room_id);
            assert!(successor_room.reason.is_none());
        });
    }

    #[async_test]
    async fn test_no_predecessor_room() {
        let client = logged_in_base_client(None).await;
        let room = client.get_or_create_room(room_id!("!r0"), RoomState::Joined);

        assert!(room.create_content().is_none());
        assert!(room.predecessor_room().is_none());
    }

    #[async_test]
    async fn test_no_predecessor_room_with_create_event() {
        let client = logged_in_base_client(None).await;
        let sender = user_id!("@mnt_io:matrix.org");
        let room_id = room_id!("!r1");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        let mut sync_builder = SyncResponseBuilder::new();
        let response = sync_builder
            .add_joined_room(
                JoinedRoomBuilder::new(room_id).add_timeline_event(
                    EventFactory::new()
                        .create(sender, RoomVersionId::V11)
                        // No `predecessor` field!
                        .no_predecessor()
                        .into_raw_sync(),
                ),
            )
            .build_sync_response();

        client.receive_sync_response(response).await.unwrap();

        assert!(room.create_content().is_some());
        assert!(room.predecessor_room().is_none());
    }

    #[async_test]
    async fn test_predecessor_room() {
        let client = logged_in_base_client(None).await;
        let sender = user_id!("@mnt_io:matrix.org");
        let room_id = room_id!("!r1");
        let predecessor_room_id = room_id!("!r0");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        let mut sync_builder = SyncResponseBuilder::new();
        let response = sync_builder
            .add_joined_room(
                JoinedRoomBuilder::new(room_id).add_timeline_event(
                    EventFactory::new()
                        .create(sender, RoomVersionId::V11)
                        .predecessor(predecessor_room_id)
                        .into_raw_sync(),
                ),
            )
            .build_sync_response();

        client.receive_sync_response(response).await.unwrap();

        assert!(room.create_content().is_some());
        assert_matches!(room.predecessor_room(), Some(predecessor_room) => {
            assert_eq!(predecessor_room.room_id, predecessor_room_id);
            // The creator of this room and our own user were both in the predecessor
            // room; their servers are the candidates, without duplicates.
            assert_eq!(
                predecessor_room.via,
                vec![
                    server_name!("matrix.org").to_owned(),
                    server_name!("e.uk").to_owned(),
                ]
            );
        });
    }

    #[test]
    fn test_via_servers_are_deduplicated_and_capped() {
        let users = [
            user_id!("@a:one.example"),
            user_id!("@b:one.example"),
            user_id!("@c:two.example"),
            user_id!("@d:three.example"),
            user_id!("@e:four.example"),
            user_id!("@f:five.example"),
            user_id!("@g:six.example"),
        ];

        assert_eq!(
            super::via_servers(users.iter().copied()),
            vec![
                server_name!("one.example").to_owned(),
                server_name!("two.example").to_owned(),
                server_name!("three.example").to_owned(),
                server_name!("four.example").to_owned(),
                server_name!("five.example").to_owned(),
            ]
        );
        assert!(super::via_servers(std::iter::empty()).is_empty());
    }
}
