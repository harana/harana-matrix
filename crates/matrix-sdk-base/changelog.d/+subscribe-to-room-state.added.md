Add `Room::subscribe_state`, a stream of the membership of our own user in a
room. It yields the current state right away and then every transition, so a
client can learn that it was invited, joined, kicked or banned without watching
the timeline or interpreting sync updates.
