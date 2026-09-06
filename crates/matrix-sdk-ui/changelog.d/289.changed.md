The notification sliding sync request asks for the current user's membership
event by its full user ID, rather than by the `$ME` special state key that
[MSC4186](https://github.com/matrix-org/matrix-spec-proposals/pull/4186)
removed.

`NotificationProcessSetup::MultipleProcesses` now carries a `lock_holder_name`,
so the value identifying the notification process in the cross-process lock can
be set by the caller instead of always being the hardcoded `"notifications"`.
Two processes sharing that value would both believe they hold the lock. Use
`NotificationClient::DEFAULT_LOCK_HOLDER_NAME` to keep the previous value.

`EncryptionSyncService::new` takes an `EncryptionSyncMode`. In
`EncryptionSyncMode::Notification`, the encryption sync no longer marks every
tracked user as dirty: a notification process starts a new sliding sync on every
push, so it never has a `pos`, and invalidating the device list cache each time
triggered a `/keys/query` for every tracked user on every single push. The app's
own encryption sync (`EncryptionSyncMode::App`) is the one that tracks device
list updates, and keeps the previous behavior.
