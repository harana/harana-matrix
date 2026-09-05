`UserIdentity::pin()` now pins the identity as the store currently holds it,
under the same lock as `/keys/query` processing, and does nothing if the stored
master key has changed since the caller obtained its copy. Writing the whole
in-memory identity back could revert cross-signing keys that had just arrived,
breaking communication with that user until the next key query.
