`SuccessorRoom` and `PredecessorRoom` now carry a `via` list of candidate
servers. A room ID is not routable on its own, so without it a user whose own
server has never seen the successor room could not follow a tombstone. The
candidates are the server of whoever tombstoned the room (newly recorded in
`RoomInfo`), the creator of the room and our own user for the predecessor, and
the servers of the room heroes.
