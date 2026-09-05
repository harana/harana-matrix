`RoomInfo::handle_stripped_state_event` now marks the encryption state as
synced when the stripped state of an invite carries an `m.room.encryption`
event, so `Room::encryption_state` reports `Encrypted` for a room we are only
invited to.
