#![allow(clippy::exhaustive_enums, clippy::exhaustive_structs)]
// Upstream these helpers also served the state resolution benchmarks, which are
// not vendored, so a few of them have no caller here. They are kept whole so the
// module stays a straight copy of upstream and can be updated as one.
#![allow(dead_code)]

use crate::__ruma::{OwnedRoomId, owned_room_id, room_version_rules::RoomIdFormatVersion};

mod factory;
mod pdu;

pub use self::{factory::*, pdu::*};

/// Get the default room ID in the proper format according to the room version
/// rules.
pub fn default_room_id(format: &RoomIdFormatVersion) -> OwnedRoomId {
    match format {
        RoomIdFormatVersion::V1 => owned_room_id!("!room:matrix.local"),
        // The default ID of the `m.room.create` event.
        RoomIdFormatVersion::V2 => owned_room_id!("!room-create"),
    }
}
