`RoomInfo` now exposes the data that was only reachable through `Room`, so a
subscriber of `Room::subscribe_info` no longer has to reach back into the room
object: `unread_notification_counts`, `num_unread_messages`,
`num_unread_notifications`, `num_unread_mentions`, `is_state_fully_synced`,
`is_state_partially_or_fully_synced`, `last_prev_batch`, `cached_display_name`,
`cached_user_defined_notification_mode`, `latest_event_value`,
`recency_stamp`, `encryption_settings`, `direct_targets`, `max_power_level`,
`is_marked_unread`, `is_favourite`, `is_low_priority`, `is_tombstoned` and
`tombstone_sender`. The accessors borrow, so `RoomInfo` carries no extra
cloning cost, and the matching `Room` methods now delegate to them.
