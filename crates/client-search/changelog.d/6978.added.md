Added the `backend` module, holding the engine-agnostic `RoomSearchIndex` and
`SearchIndexProvider` traits plus the types they exchange. The built-in Tantivy
index implements them and now lives behind the new `tantivy` feature, which is
on by default; turning it off leaves just the traits, so a client can search
with an engine of its own without pulling in Tantivy, which needs `mmap` and so
cannot be built for Wasm.
