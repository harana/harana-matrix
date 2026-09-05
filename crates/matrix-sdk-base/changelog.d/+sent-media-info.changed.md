[**breaking**] `SentMediaInfo` is now a single `medias: Vec<SentMediaItem>`
rather than the per-media `file`/`thumbnail` fields plus an `accumulated` vector
of `AccumulatedSentMediaInfo` holding the same two fields.
`AccumulatedSentMediaInfo` is renamed to `SentMediaItem` and is no longer behind
the `unstable-msc4274` feature, and `QueuedRequestKind::MediaUpload`'s
`accumulated` field becomes `uploaded`. Stored send queue requests are migrated
when they are read back.
