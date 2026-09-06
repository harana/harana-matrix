A corrupted media store or event cache store database is now deleted and
recreated from scratch instead of failing to open. SQLite reports an unreadable
database file with `database disk image is malformed`, which used to leave the
client permanently broken with no recovery path short of a reinstall. Both
stores are caches that can be refilled from the homeserver, so they now start
over rather than propagating the error. Only the two corruption result codes
(`SQLITE_CORRUPT` and `SQLITE_NOTADB`) trigger this; a transient failure such as
a busy database is still reported as an error.
