The storage backend is now pluggable through `ClientBuilder::store_provider()`.
Implement the new `StoreProvider` trait over any database and the SDK calls it
while building the `Client`, so opening the stores can be asynchronous and
fallible; a failure is reported as the new `ClientBuildError::StoreProvider`
variant. `ClientBuilder::store_config()` remains for stores that are already
open by the time the client is built.

See the new "Storage" section in the crate documentation.
