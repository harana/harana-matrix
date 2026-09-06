Add `SqliteStoreConfig::synchronous` and the `Synchronous` enum to control
`PRAGMA synchronous`. When it is not set, the state, event cache and media
stores default to `NORMAL` and the crypto store to `FULL`; `Off`, `Normal`,
`Full` and `Extra` can be selected explicitly.
