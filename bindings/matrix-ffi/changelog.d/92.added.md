Add `Room::paginated_members`, which returns one filtered and sorted page of a
room's members plus the total number of matches, so a member list no longer
crosses the FFI boundary whole, and `Room::subscribe_to_member_updates`, which
says when to ask again.
