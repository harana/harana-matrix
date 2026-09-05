`EventTimelineItem::local_edit()` reports the send state of an edit of that item
that the server hasn't acknowledged yet, along with the send queue handle to
retry or drop it. An edit has no timeline item of its own, so a failure to send
one used to be dropped with a warning: the edit stayed applied to its target,
indistinguishable from one that had been sent, and there was no way to retry it.
