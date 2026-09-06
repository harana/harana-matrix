[**breaking**] `Timeline::send_reply()` and `Timeline::send_location()` take a
`TimelineEventItemId` rather than an `OwnedEventId`, so the event being replied
to can be a local echo. The reply is then queued behind it and gets its relation
once the server has given the replied-to event an ID, which makes it possible to
reply to a message that hasn't been sent yet.
