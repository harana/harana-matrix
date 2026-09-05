[**breaking**] `SentMediaInfo` is now a single `medias: Vec<SentMediaItem>`, in
upload order, whose last entry is the media the request that produced it
uploaded. It used to hold the same data in two shapes: `file`/`thumbnail` for
that media, plus an `accumulated` vector of `AccumulatedSentMediaInfo` carrying
exactly those two fields for the media uploaded earlier in the same gallery
transaction. `AccumulatedSentMediaInfo` becomes `SentMediaItem` and is no longer
behind the gallery feature. Persisted requests are migrated on read.
