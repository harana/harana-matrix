Add `Room::member_list`, a query over the members of a room that filters by
membership, by a search term matching the display name or user ID, and by a
minimum power level; sorts by name or by power level then name; and returns one
page at a time along with the total number of matches. Add
`Room::subscribe_to_member_updates`, a stream that says when members join,
leave or change their profile, so a member list knows when to run its query
again. Filtering and sorting happen in memory over the members read from the
store, so paginating bounds what a caller has to handle, not what the store
loads.
