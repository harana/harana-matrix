Add `Room::join_successor_room` and `Room::successor_room_preview`, which join
or preview the room that replaced a tombstoned one, passing the candidate
servers of `SuccessorRoom::via` along as `via` parameters.
