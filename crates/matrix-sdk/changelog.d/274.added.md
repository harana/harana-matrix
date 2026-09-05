Add `Client::rooms_with_state_event`, which returns every known room whose
state holds an event with exactly the given type and state key. A client that
writes an idempotency marker when it creates a room can use the length of that
list to tell apart "never created", "created once" and "created more than
once" after a timeout, instead of retrying blindly.
