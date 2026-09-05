`SendHandle::reply()` queues a reply to an event that hasn't been sent yet. A
reply carries the event ID of what it replies to, and a local echo doesn't have
one, so the reply is queued behind it and gets its relation once the server has
given the replied-to event an ID. This makes it possible to reply to a message
that is still on its way, or that hasn't left the device at all because there is
no connectivity. Where the reply sits in a thread is chosen with the new
`ReplyThreading` argument.
