`SpaceRoomList` no longer publishes a room it already holds. `/hierarchy` can
report the same room again in a later page, and a `reset()` racing an in-flight
pagination replays the first page, so the list emitted duplicate entries and
consumers keying their list by room ID crashed on the duplicate key.
