`BackupMachine::sign_backup` now fails with `SignatureError::MissingSigningKey`
instead of falling back to a device-only signature when the cross-signing master
key is unavailable. A backup signed only by the device that created it stops
being verifiable once that device is deleted, leaving a later session no way to
check that the backup's auth data was not tampered with.
