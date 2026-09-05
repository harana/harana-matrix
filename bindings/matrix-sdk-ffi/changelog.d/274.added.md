`CreateRoomParameters` now takes an `initial_state` list, so custom state
events, an idempotency marker among them, are set as part of the same
`/createRoom` request as everything else. `Room::state_event` reads back the
state event with exactly a given type and state key, and
`Client::rooms_with_state_event` finds every room carrying one, which
distinguishes zero, one and several.
