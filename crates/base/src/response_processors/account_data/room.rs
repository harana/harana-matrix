// Copyright 2025 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use ruma::{
    RoomId,
    events::{
        AnyRoomAccountDataEvent, fully_read::FullyReadEventContent,
        marked_unread::MarkedUnreadEventContent,
    },
    serde::Raw,
};
use tracing::{instrument, warn};

use super::super::{Context, RoomInfoNotableUpdates};
use crate::{
    RoomInfo, RoomInfoNotableUpdateReasons, StateChanges, room::AccountDataSource,
    store::BaseStateStore,
};

#[instrument(skip_all, fields(?room_id))]
pub fn for_room(
    context: &mut Context,
    room_id: &RoomId,
    events: &[Raw<AnyRoomAccountDataEvent>],
    state_store: &BaseStateStore,
) {
    // Handle new events.
    for raw_event in events {
        match raw_event.deserialize() {
            Ok(event) => {
                context.state_changes.add_room_account_data(
                    room_id,
                    event.clone(),
                    raw_event.clone(),
                );

                match event {
                    AnyRoomAccountDataEvent::MarkedUnread(event) => {
                        on_room_info(
                            room_id,
                            &mut context.state_changes,
                            state_store,
                            |room_info| {
                                on_unread_marker(
                                    room_id,
                                    &event.content,
                                    AccountDataSource::Stable,
                                    room_info,
                                    &mut context.room_info_notable_updates,
                                );
                            },
                        );
                    }
                    AnyRoomAccountDataEvent::UnstableMarkedUnread(event) => {
                        on_room_info(
                            room_id,
                            &mut context.state_changes,
                            state_store,
                            |room_info| {
                                on_unread_marker(
                                    room_id,
                                    &event.content.0,
                                    AccountDataSource::Unstable,
                                    room_info,
                                    &mut context.room_info_notable_updates,
                                );
                            },
                        );
                    }
                    AnyRoomAccountDataEvent::Tag(event) => {
                        on_room_info(
                            room_id,
                            &mut context.state_changes,
                            state_store,
                            |room_info| {
                                room_info.base_info.handle_notable_tags(&event.content.tags);
                            },
                        );
                    }
                    AnyRoomAccountDataEvent::FullyRead(event) => {
                        on_room_info(
                            room_id,
                            &mut context.state_changes,
                            state_store,
                            |room_info| {
                                on_fully_read_marker(
                                    room_id,
                                    &event.content,
                                    room_info,
                                    &mut context.room_info_notable_updates,
                                );
                            },
                        );
                    }

                    // Nothing.
                    _ => {}
                }
            }

            Err(err) => {
                warn!("unable to deserialize account data event: {err}");
            }
        }
    }
}

// Small helper to make the code easier to read.
//
// It finds the appropriate `RoomInfo`, allowing the caller to modify it, and
// save it in the correct place.
fn on_room_info<F>(
    room_id: &RoomId,
    state_changes: &mut StateChanges,
    state_store: &BaseStateStore,
    mut on_room_info: F,
) where
    F: FnMut(&mut RoomInfo),
{
    // `StateChanges` has the `RoomInfo`.
    if let Some(room_info) = state_changes.room_infos.get_mut(room_id) {
        // Show time.
        on_room_info(room_info);
    }
    // The `BaseStateStore` has the `Room`, which has the `RoomInfo`.
    else if let Some(room) = state_store.room(room_id) {
        // Clone the `RoomInfo`.
        let mut room_info = room.clone_info();

        // Show time.
        on_room_info(&mut room_info);

        // Update the `RoomInfo` via `StateChanges`.
        state_changes.add_room(room_info);
    }
}

// Helper to update the fully-read marker event id on the `RoomInfo` and
// notify subscribers when the value changes.
fn on_fully_read_marker(
    room_id: &RoomId,
    content: &FullyReadEventContent,
    room_info: &mut RoomInfo,
    room_info_notable_updates: &mut RoomInfoNotableUpdates,
) {
    if room_info.base_info.fully_read_event_id.as_ref() == Some(&content.event_id) {
        return;
    }

    room_info.base_info.fully_read_event_id = Some(content.event_id.clone());
    room_info_notable_updates
        .entry(room_id.to_owned())
        .or_default()
        .insert(RoomInfoNotableUpdateReasons::FULLY_READ);
}

// Helper to update the unread marker for stable and unstable prefixes.
fn on_unread_marker(
    room_id: &RoomId,
    content: &MarkedUnreadEventContent,
    source: AccountDataSource,
    room_info: &mut RoomInfo,
    room_info_notable_updates: &mut RoomInfoNotableUpdates,
) {
    if room_info.base_info.is_marked_unread_source == AccountDataSource::Stable
        && source != AccountDataSource::Stable
    {
        // Ignore the unstable source if a stable source was used previously.
        return;
    }

    if room_info.base_info.is_marked_unread != content.unread {
        // Notify the room list about a manual read marker change if the
        // value's changed.
        room_info_notable_updates
            .entry(room_id.to_owned())
            .or_default()
            .insert(RoomInfoNotableUpdateReasons::UNREAD_MARKER);
    }

    room_info.base_info.is_marked_unread = content.unread;
    room_info.base_info.is_marked_unread_source = source;
}

#[cfg(test)]
mod tests {
    use ruma::{
        RoomId, event_id,
        events::{
            AnyRoomAccountDataEvent, RoomAccountDataEventType,
            tag::{TagInfo, TagName, Tags},
        },
        room_id,
        serde::Raw,
    };
    use sdk_test::{async_test, event_factory::EventFactory};
    use serde_json::json;

    use crate::{
        BaseClient, RoomInfoNotableUpdateReasons, RoomState, response_processors as processors,
        room::{AccountDataSource, RoomNotableTags},
        test_utils::logged_in_base_client,
    };

    /// Runs the room account data processor on a fresh [`Context`], and
    /// returns it so that the resulting changes can be inspected.
    fn process(
        client: &BaseClient,
        room_id: &RoomId,
        events: &[Raw<AnyRoomAccountDataEvent>],
    ) -> processors::Context {
        let mut context = processors::Context::default();

        processors::account_data::for_room(&mut context, room_id, events, &client.state_store);

        context
    }

    /// Every room account data event is kept as-is in the state changes, so
    /// that it can be persisted.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#client-config>.
    #[async_test]
    async fn test_room_account_data_events_are_kept() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        client.get_or_create_room(room_id, RoomState::Joined);

        let f = EventFactory::new();
        let context = process(
            &client,
            room_id,
            &[f.fully_read(event_id!("$1")).into_raw(), f.marked_unread(true).into_raw()],
        );

        let account_data = context.state_changes.room_account_data.get(room_id).unwrap();

        assert!(account_data.contains_key(&RoomAccountDataEventType::FullyRead));
        assert!(account_data.contains_key(&RoomAccountDataEventType::MarkedUnread));
    }

    /// An `m.fully_read` event updates the fully-read marker of the room, and
    /// is flagged as a notable update.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#fully-read-markers>.
    #[async_test]
    async fn test_fully_read_marker_is_applied() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        // Sanity check: no fully-read marker yet.
        assert!(room.clone_info().fully_read_event_id().is_none());

        let f = EventFactory::new();
        let context = process(&client, room_id, &[f.fully_read(event_id!("$1")).into_raw()]);

        let room_info = context.state_changes.room_infos.get(room_id).unwrap();
        assert_eq!(room_info.fully_read_event_id(), Some(event_id!("$1")));
        assert!(
            context
                .room_info_notable_updates
                .get(room_id)
                .unwrap()
                .contains(RoomInfoNotableUpdateReasons::FULLY_READ)
        );
    }

    /// Receiving the same `m.fully_read` marker twice must not be reported as
    /// a notable update the second time.
    #[async_test]
    async fn test_unchanged_fully_read_marker_is_not_notable() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        client.get_or_create_room(room_id, RoomState::Joined);

        let f = EventFactory::new();

        // Apply the marker a first time.
        let context = process(&client, room_id, &[f.fully_read(event_id!("$1")).into_raw()]);
        processors::changes::save_and_apply(
            context,
            &client.state_store,
            &client.state_store_lock().lock().await,
            &client.ignore_user_list_changes,
            None,
        )
        .await
        .unwrap();

        // The very same marker arrives again.
        let context = process(&client, room_id, &[f.fully_read(event_id!("$1")).into_raw()]);

        assert!(
            !context
                .room_info_notable_updates
                .get(room_id)
                .copied()
                .unwrap_or_default()
                .contains(RoomInfoNotableUpdateReasons::FULLY_READ)
        );
    }

    /// An `m.marked_unread` event sets the manual unread marker of the room,
    /// and is flagged as a notable update.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#unread-markers>.
    #[async_test]
    async fn test_marked_unread_is_applied() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        assert!(!room.is_marked_unread());

        let f = EventFactory::new();
        let context = process(&client, room_id, &[f.marked_unread(true).into_raw()]);

        let room_info = context.state_changes.room_infos.get(room_id).unwrap();
        assert!(room_info.base_info.is_marked_unread);
        assert_eq!(room_info.base_info.is_marked_unread_source, AccountDataSource::Stable);
        assert!(
            context
                .room_info_notable_updates
                .get(room_id)
                .unwrap()
                .contains(RoomInfoNotableUpdateReasons::UNREAD_MARKER)
        );
    }

    /// The unstable `com.famedly.marked_unread` event is understood too, and
    /// is recorded as coming from an unstable source.
    #[async_test]
    async fn test_unstable_marked_unread_is_applied() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        client.get_or_create_room(room_id, RoomState::Joined);

        let unstable: Raw<AnyRoomAccountDataEvent> = Raw::new(&json!({
            "type": "com.famedly.marked_unread",
            "content": { "unread": true },
        }))
        .unwrap()
        .cast_unchecked();

        let context = process(&client, room_id, &[unstable]);

        let room_info = context.state_changes.room_infos.get(room_id).unwrap();
        assert!(room_info.base_info.is_marked_unread);
        assert_eq!(room_info.base_info.is_marked_unread_source, AccountDataSource::Unstable);
    }

    /// Once the stable `m.marked_unread` event has been seen, the unstable
    /// variant must be ignored, so that a stale unstable event doesn't undo
    /// the stable one.
    #[async_test]
    async fn test_stable_marked_unread_wins_over_the_unstable_one() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        client.get_or_create_room(room_id, RoomState::Joined);

        let f = EventFactory::new();
        let unstable: Raw<AnyRoomAccountDataEvent> = Raw::new(&json!({
            "type": "com.famedly.marked_unread",
            "content": { "unread": false },
        }))
        .unwrap()
        .cast_unchecked();

        // The stable event is processed first, then the unstable one.
        let context = process(&client, room_id, &[f.marked_unread(true).into_raw(), unstable]);

        let room_info = context.state_changes.room_infos.get(room_id).unwrap();
        assert!(room_info.base_info.is_marked_unread);
        assert_eq!(room_info.base_info.is_marked_unread_source, AccountDataSource::Stable);
    }

    /// An `m.tag` event updates the notable tags of the room.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#room-tagging>.
    #[async_test]
    async fn test_tags_are_applied() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        client.get_or_create_room(room_id, RoomState::Joined);

        let f = EventFactory::new();
        let mut tags = Tags::new();
        tags.insert(TagName::Favorite, TagInfo::default());
        tags.insert(TagName::LowPriority, TagInfo::default());

        let context = process(&client, room_id, &[f.tag(tags).into_raw()]);

        let room_info = context.state_changes.room_infos.get(room_id).unwrap();
        assert!(room_info.base_info.notable_tags.contains(RoomNotableTags::FAVOURITE));
        assert!(room_info.base_info.notable_tags.contains(RoomNotableTags::LOW_PRIORITY));
    }

    /// A room account data event for an unknown room is ignored, and doesn't
    /// create a `RoomInfo` out of thin air.
    #[async_test]
    async fn test_event_for_an_unknown_room_is_ignored() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!unknown:localhost");

        let f = EventFactory::new();
        let context = process(&client, room_id, &[f.fully_read(event_id!("$1")).into_raw()]);

        // The raw event is still kept…
        assert!(context.state_changes.room_account_data.contains_key(room_id));
        // …but no `RoomInfo` was created.
        assert!(!context.state_changes.room_infos.contains_key(room_id));
    }

    /// A malformed room account data event is skipped, and doesn't prevent the
    /// following events from being processed.
    #[async_test]
    async fn test_malformed_event_is_skipped() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!r:localhost");
        client.get_or_create_room(room_id, RoomState::Joined);

        let malformed: Raw<AnyRoomAccountDataEvent> =
            Raw::new(&json!({ "type": "m.fully_read", "content": {} })).unwrap().cast_unchecked();

        let f = EventFactory::new();
        let context = process(&client, room_id, &[malformed, f.marked_unread(true).into_raw()]);

        let account_data = context.state_changes.room_account_data.get(room_id).unwrap();
        assert!(!account_data.contains_key(&RoomAccountDataEventType::FullyRead));
        assert!(account_data.contains_key(&RoomAccountDataEventType::MarkedUnread));
    }
}
