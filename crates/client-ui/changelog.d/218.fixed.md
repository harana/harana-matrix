`test_retry_decryption_updates_reply` no longer assumes the decrypted event and
the reply pointing at it are updated in a fixed order, which made it fail
intermittently on a loaded machine.
