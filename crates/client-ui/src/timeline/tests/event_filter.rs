// Copyright 2023 Kévin Commaille
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

use std::sync::Arc;

use assert_matches::assert_matches;
use assert_matches2::assert_let;
use eyeball_im::VectorDiff;
use client_matrix::deserialized_responses::TimelineEvent;
use common_test::{ALICE, BOB, async_test, sync_timeline_event};
use common_ruma::{
    events::{
        AnyMessageLikeEventContent, AnySyncTimelineEvent, MessageLikeEventType, StateEventType,
        TimelineEventType,
        reaction::ReactionEventContent,
        relation::Annotation,
        room::{
            member::MembershipState,
            message::{MessageType, RedactedRoomMessageEventContent, RoomMessageEventContent},
        },
    },
    mxc_uri,
};
use stream_assert::assert_next_matches;

use super::TestTimeline;
use crate::timeline::{
    AnyOtherStateEventContentChange, MsgLikeContent, MsgLikeKind, TimelineEventCondition,
    TimelineEventFilter, TimelineItem, TimelineItemContent, TimelineItemKind,
    controller::TimelineSettings, event_filter::MembershipChangeFilter, tests::TestTimelineBuilder,
};

#[async_test]
async fn test_default_filter() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;

    // Test edits work.
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let _date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    let first_event_id = item.as_event().unwrap().event_id().unwrap();

    timeline
        .handle_live_event(
            f.text_msg(" * The _edited_ first message")
                .sender(&ALICE)
                .edit(first_event_id, MessageType::text_plain("The _edited_ first message").into()),
        )
        .await;

    // The edit was applied.
    let item = assert_next_matches!(stream, VectorDiff::Set { index: 1, value } => value);
    assert_let!(Some(message) = item.as_event().unwrap().content().as_message());
    assert_let!(MessageType::Text(text) = message.msgtype());
    assert_eq!(text.body, "The _edited_ first message");

    // TODO: After adding raw timeline items, check for one here.

    // Test redactions work.
    timeline.handle_live_event(f.text_msg("The second message").sender(&ALICE)).await;
    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let second_event_id = item.as_event().unwrap().event_id().unwrap();

    timeline.handle_live_event(f.redaction(second_event_id).sender(&BOB)).await;
    let item = assert_next_matches!(stream, VectorDiff::Set { index: 2, value } => value);
    assert!(item.as_event().unwrap().content().is_redacted());

    // TODO: After adding raw timeline items, check for one here.

    // Test reactions work.
    timeline.handle_live_event(f.text_msg("The third message").sender(&ALICE)).await;
    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let third_event_id = item.as_event().unwrap().event_id().unwrap();

    timeline.handle_live_event(f.reaction(third_event_id, "+1").sender(&BOB)).await;
    timeline.handle_live_event(f.redaction(second_event_id).sender(&BOB)).await;
    let item = assert_next_matches!(stream, VectorDiff::Set { index: 3, value } => value);
    assert_eq!(
        item.as_event().unwrap().content().reactions().cloned().unwrap_or_default().len(),
        1
    );

    // TODO: After adding raw timeline items, check for one here.

    assert_eq!(timeline.controller.items().await.len(), 4);
}

#[async_test]
async fn test_filter_always_false() {
    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings { event_filter: Arc::new(|_, _| false), ..Default::default() })
        .build()
        .await;

    let f = &timeline.factory;
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;

    timeline.handle_live_event(f.redacted(&ALICE, RedactedRoomMessageEventContent::new())).await;

    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;

    timeline.handle_live_event(f.room_name("Alice's room").sender(&ALICE)).await;

    assert_eq!(timeline.controller.items().await.len(), 0);
}

#[async_test]
async fn test_local_echoes_are_filtered_too() {
    // A local echo goes through the same filter as a remote event: a timeline that
    // only shows notices must not show a text message just because it hasn't been
    // sent yet.
    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(|event, _| {
                assert_let!(AnySyncTimelineEvent::MessageLike(event) = event);
                assert_let!(
                    Some(AnyMessageLikeEventContent::RoomMessage(content)) =
                        event.original_content()
                );
                matches!(content.msgtype, MessageType::Notice(_))
            }),
            ..Default::default()
        })
        .build()
        .await;

    timeline.handle_local_event(RoomMessageEventContent::text_plain("filtered out").into()).await;
    assert!(timeline.controller.items().await.is_empty());

    timeline.handle_local_event(RoomMessageEventContent::notice_plain("kept").into()).await;

    let items = timeline.controller.items().await;
    assert_eq!(items.len(), 2);
    assert!(items[0].is_date_divider());
    assert_let!(Some(event) = items[1].as_event());
    assert!(event.is_local_echo());
}

#[async_test]
async fn test_local_echo_aggregations_are_not_filtered_out() {
    // The filter decides whether a local echo becomes an item of its own; it must
    // not stop an aggregation from being applied to the item it targets. A
    // reaction is filtered out by the default filter, yet it still shows up on
    // its target.
    let timeline = TestTimeline::new().await;

    let f = &timeline.factory;
    timeline.handle_live_event(f.text_msg("hello").sender(&ALICE)).await;

    let event_id =
        timeline.controller.items().await[1].as_event().unwrap().event_id().unwrap().to_owned();

    timeline
        .handle_local_event(
            ReactionEventContent::new(Annotation::new(event_id, "👍".to_owned())).into(),
        )
        .await;

    let items = timeline.controller.items().await;
    // Still only the date divider and the message: no item for the reaction.
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[1].as_event().unwrap().content().reactions().cloned().unwrap_or_default().len(),
        1
    );
}

#[async_test]
async fn test_custom_filter() {
    // Filter out all state events.
    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(|ev, _| matches!(ev, AnySyncTimelineEvent::MessageLike(_))),
            ..Default::default()
        })
        .build()
        .await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    let _item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let _date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);

    timeline.handle_live_event(f.redacted(&ALICE, RedactedRoomMessageEventContent::new())).await;
    let _item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);

    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;

    timeline.handle_live_event(f.room_name("Alice's room").sender(&ALICE)).await;

    assert_eq!(timeline.controller.items().await.len(), 3);
}

#[async_test]
async fn test_custom_filter_for_custom_msglike_event() {
    // Filter out all state events.
    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(|ev, _| matches!(ev, AnySyncTimelineEvent::MessageLike(_))),
            ..Default::default()
        })
        .build()
        .await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;
    timeline.handle_live_event(f.custom_message_like_event().sender(&ALICE)).await;
    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);

    assert_matches!(
        item.as_event().unwrap().content().as_msglike().unwrap().kind.clone(),
        MsgLikeKind::Other(_)
    );
    assert!(date_divider.is_date_divider());

    assert_eq!(timeline.controller.items().await.len(), 2);
}

#[async_test]
async fn test_hide_failed_to_parse() {
    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings { add_failed_to_parse: false, ..Default::default() })
        .build()
        .await;

    // m.room.message events must have a msgtype and body in content, so this
    // event with an empty content object should fail to deserialize.
    timeline
        .handle_live_event(TimelineEvent::from_plaintext(sync_timeline_event!({
            "content": {},
            "event_id": "$eeG0HA0FAZ37wP8kXlNkxx3I",
            "origin_server_ts": 10,
            "sender": "@alice:example.org",
            "type": "m.room.message",
        })))
        .await;

    // Similar to above, the m.room.member state event must also not have an
    // empty content object.
    timeline
        .handle_live_event(TimelineEvent::from_plaintext(sync_timeline_event!({
            "content": {},
            "event_id": "$d5G0HA0FAZ37wP8kXlNkxx3I",
            "origin_server_ts": 2179,
            "sender": "@alice:example.org",
            "type": "m.room.member",
            "state_key": "@alice:example.org",
        })))
        .await;

    assert_eq!(timeline.controller.items().await.len(), 0);
}

#[async_test]
async fn test_event_filter_include_only_room_names() {
    // Only return room name events
    let event_filter = TimelineEventFilter::Include(vec![TimelineEventCondition::EventType(
        TimelineEventType::RoomName,
    )]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add a non-encrypted message event
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    // Add a couple of room name events
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name (again)").sender(&ALICE)).await;
    // And a different state event
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;

    // The timeline should contain only the room name events
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    assert_eq!(event_items.len(), 2);
    assert_eq!(num_text_message_items, 0);
    assert_eq!(num_room_name_items, 2);
    assert_eq!(num_room_topic_items, 0);
}

#[async_test]
async fn test_event_filter_exclude_messages() {
    // Don't return any messages
    let event_filter = TimelineEventFilter::Exclude(vec![TimelineEventCondition::EventType(
        TimelineEventType::RoomMessage,
    )]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add a message event
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    // Add a couple of room name state events
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name (again)").sender(&ALICE)).await;
    // And a different state event
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;

    // The timeline should contain everything except for the message event.
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    assert_eq!(event_items.len(), 3);
    assert_eq!(num_text_message_items, 0);
    assert_eq!(num_room_name_items, 2);
    assert_eq!(num_room_topic_items, 1);
}

#[async_test]
async fn test_event_filter_include_only_membership_changes() {
    // Only return room name events
    let event_filter =
        TimelineEventFilter::Include(vec![TimelineEventCondition::MembershipChange(
            MembershipChangeFilter::Any,
        )]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob joins
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline.handle_live_event(f.member(&BOB).previous(MembershipState::Invite)).await;
    // Bob changes his display name
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;

    // The timeline should contain only the invite and join events
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    assert_eq!(event_items.len(), 3);
    assert_eq!(num_text_message_items, 0);
    assert_eq!(num_room_name_items, 0);
    assert_eq!(num_room_topic_items, 0);
    assert_eq!(num_membership_change_items, 3);
    assert_eq!(num_profile_change_items, 0);
}

#[async_test]
async fn test_event_filter_include_only_profile_changes() {
    // Only return room name events
    let event_filter = TimelineEventFilter::Include(vec![TimelineEventCondition::ProfileChange]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob joins
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline.handle_live_event(f.member(&BOB).previous(MembershipState::Invite)).await;
    // Bob changes his display name
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;

    // The timeline should contain only the display name and avatar URL changes
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    assert_eq!(event_items.len(), 2);
    assert_eq!(num_text_message_items, 0);
    assert_eq!(num_room_name_items, 0);
    assert_eq!(num_room_topic_items, 0);
    assert_eq!(num_membership_change_items, 0);
    assert_eq!(num_profile_change_items, 2);
}

#[async_test]
async fn test_event_filter_include_only_messages_and_membership_changes() {
    // Only return room name events
    let event_filter = TimelineEventFilter::Include(vec![
        TimelineEventCondition::EventType(TimelineEventType::RoomMessage),
        TimelineEventCondition::MembershipChange(MembershipChangeFilter::Any),
    ]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob joins
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline.handle_live_event(f.member(&BOB).previous(MembershipState::Invite)).await;
    // Bob changes his display name
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;

    // The timeline should contain only the message, invite and join events
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    assert_eq!(event_items.len(), 4);
    assert_eq!(num_text_message_items, 1);
    assert_eq!(num_room_name_items, 0);
    assert_eq!(num_room_topic_items, 0);
    assert_eq!(num_membership_change_items, 3);
    assert_eq!(num_profile_change_items, 0);
}

#[async_test]
async fn test_event_filter_exclude_membership_changes() {
    // Only return room name events
    let event_filter =
        TimelineEventFilter::Exclude(vec![TimelineEventCondition::MembershipChange(
            MembershipChangeFilter::Any,
        )]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob joins
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline.handle_live_event(f.member(&BOB).previous(MembershipState::Invite)).await;
    // Bob changes his display name
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;

    // The timeline should contain everything except for the invite and join events
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    assert_eq!(event_items.len(), 5);
    assert_eq!(num_text_message_items, 1);
    assert_eq!(num_room_name_items, 1);
    assert_eq!(num_room_topic_items, 1);
    assert_eq!(num_membership_change_items, 0);
    assert_eq!(num_profile_change_items, 2);
}

#[async_test]
async fn test_event_filter_exclude_profile_changes() {
    // Only return room name events
    let event_filter = TimelineEventFilter::Exclude(vec![TimelineEventCondition::ProfileChange]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob joins
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline.handle_live_event(f.member(&BOB).previous(MembershipState::Invite)).await;
    // Bob changes his display name
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;

    // The timeline should contain everything except for the display name and avatar
    // URL changes
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    assert_eq!(event_items.len(), 6);
    assert_eq!(num_text_message_items, 1);
    assert_eq!(num_room_name_items, 1);
    assert_eq!(num_room_topic_items, 1);
    assert_eq!(num_membership_change_items, 3);
    assert_eq!(num_profile_change_items, 0);
}

#[async_test]
async fn test_event_filter_exclude_messages_and_membership_changes() {
    // Only return room name events
    let event_filter = TimelineEventFilter::Exclude(vec![
        TimelineEventCondition::EventType(TimelineEventType::RoomMessage),
        TimelineEventCondition::MembershipChange(MembershipChangeFilter::Any),
    ]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob joins
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline.handle_live_event(f.member(&BOB).previous(MembershipState::Invite)).await;
    // Bob changes his display name
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;

    // The timeline should contain everything except for the message, invite and
    // join events
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    assert_eq!(event_items.len(), 4);
    assert_eq!(num_text_message_items, 0);
    assert_eq!(num_room_name_items, 1);
    assert_eq!(num_room_topic_items, 1);
    assert_eq!(num_membership_change_items, 0);
    assert_eq!(num_profile_change_items, 2);
}

#[async_test]
async fn test_event_filter_can_exclude_only_join_and_leave_membership_changes() {
    let event_filter = TimelineEventFilter::Exclude(vec![
        TimelineEventCondition::MembershipChange(MembershipChangeFilter::Join),
        TimelineEventCondition::MembershipChange(MembershipChangeFilter::Leave),
    ]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add Alice's join event
    timeline.handle_live_event(f.member(&ALICE).membership(MembershipState::Join)).await;
    // Alice changes her avatar
    timeline
        .handle_live_event(
            f.member(&ALICE)
                .avatar_url(mxc_uri!("mxc://example.org/SEsfnsuifSDFSSEF"))
                .previous(MembershipState::Join),
        )
        .await;
    // Alice sends a message and changes the room name and topic
    timeline.handle_live_event(f.text_msg("The first message").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;
    // Alice invites Bob and Bob changes his display name and leaves
    timeline.handle_live_event(f.member(&ALICE).invited(&BOB)).await;
    timeline
        .handle_live_event(
            f.member(&BOB).display_name("Big Bob 99").previous(MembershipState::Join),
        )
        .await;
    timeline.handle_live_event(f.member(&BOB).leave().previous(MembershipState::Invite)).await;

    // The timeline should contain everything except for the message, invite and
    // join events
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    let num_membership_change_items = event_items.iter().filter(is_membership_change_item).count();
    let num_profile_change_items = event_items.iter().filter(is_profile_change_item).count();
    // 2 profile changes + 1 text message + 1 room name + 1 room topic + 1 invited
    // membership change
    assert_eq!(event_items.len(), 6);
    assert_eq!(num_text_message_items, 1);
    assert_eq!(num_room_name_items, 1);
    assert_eq!(num_room_topic_items, 1);
    assert_eq!(num_membership_change_items, 1);
    assert_eq!(num_profile_change_items, 2);
}

#[async_test]
async fn test_event_filter_exclude_any_custom_message_like_event_type() {
    // Don't return any custom message-like events
    let event_filter =
        TimelineEventFilter::Exclude(vec![TimelineEventCondition::AnyCustomMessageLikeEvent]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add a normal message event that will be kept
    timeline.handle_live_event(f.text_msg("Hey").sender(&ALICE)).await;
    // And a custom message event that will be filtered out
    timeline.handle_live_event(f.custom_message_like_event().sender(&ALICE)).await;
    // Add a couple of room name state events
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name (again)").sender(&ALICE)).await;
    // And a different state event
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;

    // The timeline should contain everything except for the message event.
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_normal_text_message_items = event_items.iter().filter(is_text_message_item).count();
    let num_custom_text_message_items =
        event_items.iter().filter(is_custom_text_message_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    assert_eq!(event_items.len(), 4);
    assert_eq!(num_normal_text_message_items, 1);
    assert_eq!(num_custom_text_message_items, 0);
    assert_eq!(num_room_name_items, 2);
    assert_eq!(num_room_topic_items, 1);
}

#[async_test]
async fn test_event_filter_exclude_any_custom_state_event_type() {
    // Don't return any custom state events
    let event_filter =
        TimelineEventFilter::Exclude(vec![TimelineEventCondition::AnyCustomStateEvent]);

    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(move |event, _| event_filter.filter(event)),
            ..Default::default()
        })
        .build()
        .await;
    let f = &timeline.factory;

    // Add customs state event that will be filtered out
    timeline.handle_live_event(f.custom_state_event().sender(&ALICE)).await;
    // Add a couple of room name state events
    timeline.handle_live_event(f.room_name("A new room name").sender(&ALICE)).await;
    timeline.handle_live_event(f.room_name("A new room name (again)").sender(&ALICE)).await;
    // And a different state event
    timeline.handle_live_event(f.room_topic("A new room topic").sender(&ALICE)).await;

    // The timeline should contain everything except for the message event.
    let event_items: Vec<Arc<TimelineItem>> = timeline.get_event_items().await;
    let num_custom_state_items = event_items.iter().filter(is_custom_state_item).count();
    let num_room_name_items = event_items.iter().filter(is_room_name_item).count();
    let num_room_topic_items = event_items.iter().filter(is_room_topic_item).count();
    assert_eq!(event_items.len(), 3);
    assert_eq!(num_custom_state_items, 0);
    assert_eq!(num_room_name_items, 2);
    assert_eq!(num_room_topic_items, 1);
}

impl TestTimeline {
    async fn get_event_items(&self) -> Vec<Arc<TimelineItem>> {
        self.controller
            .items()
            .await
            .into_iter()
            .filter(|i| matches!(i.kind, TimelineItemKind::Event(_)))
            .collect()
    }
}

fn is_text_message_item(item: &&Arc<TimelineItem>) -> bool {
    match item.kind() {
        TimelineItemKind::Event(event) => match &event.content {
            TimelineItemContent::MsgLike(MsgLikeContent {
                kind: MsgLikeKind::Message(message),
                ..
            }) => {
                matches!(message.msgtype, MessageType::Text(_))
            }
            _ => false,
        },
        _ => false,
    }
}

fn is_custom_text_message_item(item: &&Arc<TimelineItem>) -> bool {
    let msg_like = item.as_event().and_then(|e| e.content.as_msglike()).map(|e| e.kind.clone());
    match msg_like {
        Some(MsgLikeKind::Other(other)) => {
            matches!(other.event_type, MessageLikeEventType::_Custom(_))
        }
        _ => false,
    }
}

fn is_custom_state_item(item: &&Arc<TimelineItem>) -> bool {
    let event_type = item
        .as_event()
        .and_then(|e| match e.content() {
            TimelineItemContent::OtherState(state) => Some(state.content()),
            _ => None,
        })
        .map(|e| e.event_type());
    matches!(event_type, Some(StateEventType::_Custom(_)))
}

fn is_room_name_item(item: &&Arc<TimelineItem>) -> bool {
    match item.kind() {
        TimelineItemKind::Event(event) => match &event.content {
            TimelineItemContent::OtherState(state) => {
                matches!(state.content, AnyOtherStateEventContentChange::RoomName(_))
            }
            _ => false,
        },
        _ => false,
    }
}

fn is_room_topic_item(item: &&Arc<TimelineItem>) -> bool {
    match item.kind() {
        TimelineItemKind::Event(event) => match &event.content {
            TimelineItemContent::OtherState(state) => {
                matches!(state.content, AnyOtherStateEventContentChange::RoomTopic(_))
            }
            _ => false,
        },
        _ => false,
    }
}

fn is_membership_change_item(item: &&Arc<TimelineItem>) -> bool {
    match item.kind() {
        TimelineItemKind::Event(event) => {
            matches!(&event.content, TimelineItemContent::MembershipChange(_))
        }
        _ => false,
    }
}

fn is_profile_change_item(item: &&Arc<TimelineItem>) -> bool {
    match item.kind() {
        TimelineItemKind::Event(event) => {
            matches!(&event.content, TimelineItemContent::ProfileChange(_))
        }
        _ => false,
    }
}

#[async_test]
async fn test_event_filter_applies_to_local_echoes() {
    // A filter that only lets notices through.
    let timeline = TestTimelineBuilder::new()
        .settings(TimelineSettings {
            event_filter: Arc::new(|event, _| {
                let AnySyncTimelineEvent::MessageLike(msg) = event else { return false };
                matches!(
                    msg.original_content(),
                    Some(AnyMessageLikeEventContent::RoomMessage(content))
                        if matches!(content.msgtype, MessageType::Notice(_))
                )
            }),
            ..Default::default()
        })
        .build()
        .await;

    // A local echo the filter excludes doesn't get an item…
    timeline
        .handle_local_event(AnyMessageLikeEventContent::RoomMessage(
            RoomMessageEventContent::text_plain("filtered out"),
        ))
        .await;
    assert_eq!(timeline.controller.items().await.len(), 0);

    // …while one the filter allows does.
    timeline
        .handle_local_event(AnyMessageLikeEventContent::RoomMessage(
            RoomMessageEventContent::notice_plain("let through"),
        ))
        .await;

    let items = timeline.controller.items().await;
    assert_eq!(items.len(), 2); // the local echo, and its date divider
    let event = items[1].as_event().unwrap();
    assert!(event.is_local_echo());
    assert_let!(Some(message) = event.content().as_message());
    assert_matches!(message.msgtype(), MessageType::Notice(_));
}
