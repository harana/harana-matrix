`QueuedRequestKind` gains `ReadReceipt`, `ReadMarkers` and `UnreadMarker`
variants, so that what the client says about a room on the user's behalf can go
through the send queue instead of being lost when it can't be sent right away.
`QueuedRequestKind::is_order_sensitive()` tells the queue that these must not
hold a room's events back, and `supersedes_key()` which of them make an earlier
pending request of the same kind pointless. `SentRequestKey::Nothing` marks a
request that succeeded without producing anything another request could depend
on.
