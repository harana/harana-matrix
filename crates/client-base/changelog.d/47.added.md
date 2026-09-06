Add `Room::subscribe_to_state()`, a stream of the current user's membership in a
room. It yields the current `RoomState` and then every transition — invited,
knocked, joined, left (which is also what being kicked looks like), banned —
without repeating an unchanged state. It is driven by the state store, so it is
a reliable way to learn that the current user is no longer in a room, unlike
watching timeline events or sliding sync updates.
