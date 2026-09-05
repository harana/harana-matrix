A megolm session restored from a key backup keeps its `legacy_session` flag when
its sender data is recomputed after a `/keys/query`. The recomputation used to
reset the flag to `false`, which hid the session's messages once insecure
devices were excluded.
