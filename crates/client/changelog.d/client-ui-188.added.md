Add `Timeline::send_reply_to`, which takes a `TimelineEventItemId` so a caller
can reply to a timeline item without knowing whether it is local or remote yet.
