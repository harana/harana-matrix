`BaseClient::receive_all_members` now recomputes the joined and invited member
counts of the room summary from the member list it received. A lazy-loading
server may never send a room summary, which left
`Room::active_members_count` at `0` while `Room::members` listed dozens of
members. The counts are also documented now, so it is clear when they are
filled in.
