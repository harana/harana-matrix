The event cache now handles every `m.receipt` ephemeral event in a sync
response, instead of only the first one. A sync can carry one receipt event
holding the room's unthreaded and main-timeline receipts plus one per thread,
so stopping at the first one silently dropped thread receipts and left thread
unread counts stale.
