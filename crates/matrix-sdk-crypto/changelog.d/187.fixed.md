Room keys imported from a server-side key backup are now marked according to
whether the backup could be authenticated. `Store::import_backed_up_room_keys`
takes a `BackupAuthenticity`, and keys from a backup with no signature we trust
are no longer grandfathered as legacy sessions, so a client asking for
cross-signed senders or legacy sessions refuses to show their messages instead
of showing them behind a warning.

`SignatureState::signed()` returned `false` for every state: it compared the
same value against two different variants with `&&`.
