`DependentQueuedRequestKind::ReplyEvent` carries a reply to an event that hasn't
been sent yet, so its relation can be filled in once the replied-to event has an
event ID. How it should sit in a thread is described by the new `ReplyThreading`
enum, a serializable counterpart of `matrix_sdk::room::reply::EnforceThread`.
