`RoomList` now re-evaluates its filters and sorters for the rooms that are
tombstoned in favour of a room that just received an update. The
`deduplicate_versions` filter hides a tombstoned room based on the state of its
successor room; nothing used to notify the room list when that successor was
joined while the list was running, so both room versions stayed visible until
the next restart.
