Add `RoomInfo::update_name` and `RoomInfo::update_topic`, the counterparts of
`RoomInfo::update_avatar` for data that did not come from a state event.
`update_name` drops the cached display name, which is computed from the name.
`RoomInfo::update_joined_member_count` and
`RoomInfo::update_invited_member_count` are now public.
