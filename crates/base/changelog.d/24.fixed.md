`BaseClient::room_joined()`, `room_left()` and `room_knocked()` now read the
room's state under the state store lock instead of before taking it. Sync
processing holds the same lock while it writes, so a sync response could land
between the check and the update and then be overwritten with the state read
before it, losing the transition.
