Add `EncryptionSyncService::new_for_notifications()`, used by the notification
client. Every push notification used to run an encryption sync with a fresh
sliding sync session, which marked all tracked users as dirty and triggered a
`/keys/query` for every one of them; the application's own sync remains
responsible for keeping the device lists up to date.
