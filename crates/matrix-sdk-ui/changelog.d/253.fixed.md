`EventTimelineItem::latest_edit_json` no longer returns a stale edit when
several edits of the same event are chained. Local echoes of edits are ordered
by the last one recorded rather than the first, and an edit with no known
position in the timeline, such as a bundled one, is ordered by its
`origin_server_ts` instead of disabling the comparison.
