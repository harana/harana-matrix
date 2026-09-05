A one-time key count from a sync response that raced our key upload is now
ignored. The upload response would report the keys as published and the sync
that was already in flight would report zero, so the SDK decided the server had
nothing and uploaded a second full batch, ending up with 100 keys instead of 50.
Counts are believed again once the server confirms it has caught up.
