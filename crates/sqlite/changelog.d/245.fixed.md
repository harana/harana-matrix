`SqliteStateStore::save_changes` no longer fails when it redacts a state event
whose content cannot be deserialized. A homeserver accepts any content for any
event type, so for instance an `m.space.child` without the required `via` field
can legitimately be in the store; redacting it used to abort the whole
transaction, which made every subsequent sync fail with the same error.
