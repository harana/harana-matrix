Sync v2 no longer writes its `next_batch` into the crypto store slot that holds
the to-device stream token of MSC4186 sliding sync, and tokens written by the
sliding sync path are now tagged. Upgrading an existing store from sync v2 to
sliding sync used to hand the server a sync v2 token as the to-device `since`,
which Synapse rejects, so every sync failed until the store was deleted and the
devices re-verified. An untagged token left by an older SDK is ignored, and the
to-device extension simply starts without a `since`.
