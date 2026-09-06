`RelationalLinkedChunk::apply_updates` now rejects a `NewItemsChunk` or
`NewGapChunk` update whose chunk identifier is already in use in that linked
chunk, returning `RelationalLinkedChunkError::DuplicateChunkIdentifier`
instead of storing a duplicate chunk. The insert is rejected before any
relinking happens, so the store is left untouched.
