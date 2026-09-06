IndexedDB transactions in the crypto store ask for strict durability. Relaxed
durability, the default, reports a write as committed while it is still in the
browser's buffers, so a crash could leave a persisted Olm session behind the one
the peer had already seen and wedge it.
