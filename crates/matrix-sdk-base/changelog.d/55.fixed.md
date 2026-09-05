The room member counts (`Room::joined_members_count()`,
`Room::invited_members_count()`, `Room::active_members_count()`) are now updated
from a complete `/members` response. Servers only send an `m.room.summary` when
the counts change, so a room the client never received a summary for reported
zero members even after its full member list had been fetched. The accessors now
also document when the count is known.
