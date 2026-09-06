`SqliteStoreConfig` gained `cipher_provider`, `value_codec` and `json_codec`,
so a store can be encrypted with a cipher of your own and can write a
serialization format of your own. The defaults are unchanged, so existing
databases keep reading and writing the same bytes.

Added the `log_targets` module, naming the `tracing` targets each SQLite store
logs under, so clients that let users tune log levels per component no longer
have to spell out module paths.
