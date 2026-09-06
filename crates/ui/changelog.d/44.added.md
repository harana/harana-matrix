Add `Timeline::download_media`, which downloads the media of a timeline event
and reports how far along the transfer is on the timeline item itself:
`EventTimelineItem::media_download_progress` is updated as bytes arrive and the
timeline emits an update for the item, so a client can show per-message
download status without wiring up a separate channel. The progress is cleared
once the transfer ends, and content served from the media cache reports none,
since there is no transfer to watch.
