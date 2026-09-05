[**breaking**] The `sqlite` feature is no longer enabled by default. SQLite is
one of the store backends the SDK ships, not a requirement: with no store
configured the SDK keeps everything in memory. Add `features = ["sqlite"]` (or
`["bundled-sqlite"]`) to keep `ClientBuilder::sqlite_store()` and the
`Sqlite*Store` types available.
