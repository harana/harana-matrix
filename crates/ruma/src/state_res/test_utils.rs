// Some of these helpers are only used by the integration tests of the crate
// this module was vendored from, which are not vendored here.
#![allow(dead_code)]
#![allow(clippy::exhaustive_enums, clippy::exhaustive_structs)]

use crate::{OwnedRoomId, owned_room_id, room_version_rules::RoomIdFormatVersion};

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
        // `RoomIdFormatVersion` is only `non_exhaustive` to other crates, and
        // this module is now inside the crate that defines it, so this arm is
        // unreachable until a version adds a format.
        #[allow(unreachable_patterns)]
        _ => panic!("Unsupported RoomIdFormatVersion"),
    }
}
