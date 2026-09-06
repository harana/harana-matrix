The `Account` is no longer written back to the crypto store on every
`process_sync_changes` call. It now tracks whether it was actually modified, and
a store transaction that only read the account hands it straight back to the
cache. Re-pickling the account several times per sync was measurable, especially
on IndexedDB.
