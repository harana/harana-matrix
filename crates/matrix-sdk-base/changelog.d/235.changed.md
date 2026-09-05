`RoomMember::display_name` and `RoomMember::avatar_url` now return `None` for a
banned member, and `RoomMember::name` falls back to the localpart of their user
ID, so a banned user is no longer shown with the profile they had when they
were still in the room. The new `RoomMember::is_banned` reports that case, and
the raw member event stays available through `RoomMember::event`.
