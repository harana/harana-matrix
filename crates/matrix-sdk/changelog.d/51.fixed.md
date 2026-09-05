`Room::request_encryption_state` no longer sends a `/state` request for a room
we are only invited to. The server answers such a request with `M_FORBIDDEN`,
so `Room::latest_encryption_state` used to fail for every invited room. The
encryption state now comes from the stripped state of the invite, and stays
`EncryptionState::Unknown` if the stripped state doesn't carry an
`m.room.encryption` event.
