In WAL mode, SQLite's compiled-in default for `PRAGMA synchronous` is `FULL`,
meaning `fsync` is called on every commit. This can be very slow on some
storage (e.g. spinning disks or RAID arrays), where a single logical write can
be amplified into several physical `fsync` calls.

A new `SqliteStoreConfig::synchronous` method allows overriding this value.
When not set, the state, event cache and media stores now default to
`NORMAL`, since they only hold data that can be re-synchronized from the
server; the crypto store keeps the durable `FULL` default, since losing
encryption keys is not recoverable.
