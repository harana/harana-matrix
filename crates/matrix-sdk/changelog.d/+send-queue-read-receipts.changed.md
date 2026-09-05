Read receipts and unread markers now go through the send queue, with
`RoomSendQueue::send_read_receipt()`, `send_read_markers()` and
`set_unread_marker()`. They used to be sent directly, and a failure - being
offline, most of the time - simply lost them, leaving a room the user had read
looking unread once connectivity was back.

Each of them supersedes the request of its own kind that is still waiting to be
sent, since only the most recent one says anything useful, and one that fails
unrecoverably is dropped rather than wedged, so it never holds a room's messages
back. The unread marker is applied locally as soon as it is queued.

As a result, `Room::send_single_receipt()`, `Room::send_multiple_receipts()` and
`Room::set_unread_flag()` return once the request is queued, not once the server
has it.
