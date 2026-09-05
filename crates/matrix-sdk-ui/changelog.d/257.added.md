The `SyncService` now backs off before restarting the underlying syncs after a
failure, instead of restarting them immediately. The delay doubles with each
consecutive failure (1s, 2s, 4s, … capped at 64s) and is reset once the syncs
have run for a while without failing. A new `State::Backoff` (`SyncServiceState.Backoff`
over FFI) is observable while the service is waiting; `SyncService::start()`
aborts the wait and syncs immediately.
