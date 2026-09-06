// Copyright 2023 The Matrix.org Foundation C.I.C.
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

use std::{collections::BTreeMap, sync::Arc};

use assert_matches::assert_matches;
use assert_matches2::assert_let;
use eyeball_im::VectorDiff;
use matrix_sdk::{
    deserialized_responses::{AlgorithmInfo, EncryptionInfo, VerificationLevel, VerificationState},
    send_queue::RoomSendQueueUpdate,
};
use matrix_sdk_base::{
    deserialized_responses::{DecryptedRoomEvent, TimelineEvent},
    store::QueueWedgeError,
};
use matrix_sdk_test::{ALICE, BOB, async_test};
use ruma::{
    event_id,
    events::{
        AnyMessageLikeEventContent,
        relation::Replacement,
        room::message::{
            MessageType, RedactedRoomMessageEventContent, Relation, ReplacementMetadata,
            RoomMessageEventContent, RoomMessageEventContentWithoutRelation,
        },
    },
    room_id,
};
use stream_assert::{assert_next_matches, assert_pending};

use super::TestTimeline;
use crate::timeline::EventSendState;

#[async_test]
async fn test_live_redacted() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;

    timeline.handle_live_event(f.redacted(*ALICE, RedactedRoomMessageEventContent::new())).await;
    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);

    let redacted_event_id = item.as_event().unwrap().event_id().unwrap();

    timeline
        .handle_live_event(
            f.text_msg(" * test")
                .sender(&ALICE)
                .edit(redacted_event_id, MessageType::text_plain("test").into()),
        )
        .await;

    assert_eq!(timeline.controller.items().await.len(), 2);

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());
}

#[async_test]
async fn test_live_sanitized() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;
    timeline
        .handle_live_event(
            f.text_html("**original** message", "<strong>original</strong> message").sender(&ALICE),
        )
        .await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let first_event = item.as_event().unwrap();
    assert_let!(Some(message) = first_event.content().as_message());
    assert_let!(MessageType::Text(text) = message.msgtype());
    assert_eq!(text.body, "**original** message");
    assert_eq!(text.formatted.as_ref().unwrap().body, "<strong>original</strong> message");

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());

    let first_event_id = first_event.event_id().unwrap();

    let new_plain_content = "!!edited!! **better** message";
    let new_html_content = "<edited/> <strong>better</strong> message";
    timeline
        .handle_live_event(
            f.text_html(format!("* {new_plain_content}"), format!("* {new_html_content}"))
                .sender(&ALICE)
                .edit(
                    first_event_id,
                    MessageType::text_html(new_plain_content, new_html_content).into(),
                ),
        )
        .await;

    let item = assert_next_matches!(stream, VectorDiff::Set { index: 1, value } => value);
    let first_event = item.as_event().unwrap();
    assert_let!(Some(message) = first_event.content().as_message());
    assert_let!(MessageType::Text(text) = message.msgtype());
    assert_eq!(text.body, new_plain_content);
    assert_eq!(text.formatted.as_ref().unwrap().body, " <strong>better</strong> message");
}

#[async_test]
async fn test_aggregated_sanitized() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let original_event_id = event_id!("$original");
    let edit_event_id = event_id!("$edit");

    let f = &timeline.factory;

    let ev = f
        .text_html("**original** message", "<strong>original</strong> message")
        .sender(*ALICE)
        .event_id(original_event_id)
        .with_bundled_edit(
            f.text_html(
                "* !!edited!! **better** message",
                "* <edited/> <strong>better</strong> message",
            )
            .edit(
                original_event_id,
                MessageType::text_html(
                    "!!edited!! **better** message",
                    "<edited/> <strong>better</strong> message",
                )
                .into(),
            )
            .event_id(edit_event_id)
            .sender(*ALICE),
        );

    timeline.handle_live_event(ev).await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let first_event = item.as_event().unwrap();
    assert_let!(Some(message) = first_event.content().as_message());
    assert_let!(MessageType::Text(text) = message.msgtype());
    assert_eq!(text.body, "!!edited!! **better** message");
    assert_eq!(text.formatted.as_ref().unwrap().body, " <strong>better</strong> message");

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());
}

#[async_test]
async fn test_edit_updates_encryption_info() {
    let timeline = TestTimeline::new().await;
    let event_factory = &timeline.factory;

    let room_id = room_id!("!room:id");
    let original_event_id = event_id!("$original_event");

    let original_event = event_factory
        .text_msg("**original** message")
        .sender(*ALICE)
        .event_id(original_event_id)
        .room(room_id)
        .into_raw();

    let mut encryption_info = Arc::new(EncryptionInfo {
        sender: (*ALICE).into(),
        sender_device: None,
        forwarder: None,
        algorithm_info: AlgorithmInfo::MegolmV1AesSha2 {
            curve25519_key: "123".to_owned(),
            sender_claimed_keys: BTreeMap::new(),
            session_id: Some("mysessionid6333".to_owned()),
        },
        verification_state: VerificationState::Verified,
    });

    let original_event = TimelineEvent::from_decrypted(
        DecryptedRoomEvent {
            event: original_event,
            encryption_info: encryption_info.clone(),
            unsigned_encryption_info: None,
        },
        None,
    );

    timeline.handle_live_event(original_event).await;

    let items = timeline.controller.items().await;
    let first_event = items[1].as_event().unwrap();

    assert_eq!(
        first_event.encryption_info().unwrap().verification_state,
        VerificationState::Verified
    );

    assert_let!(Some(message) = first_event.content().as_message());
    assert_let!(MessageType::Text(text) = message.msgtype());
    assert_eq!(text.body, "**original** message");

    let edit_event = event_factory
        .text_msg(" * !!edited!! **better** message")
        .sender(*ALICE)
        .room(room_id)
        .edit(original_event_id, MessageType::text_plain("!!edited!! **better** message").into())
        .into_raw();
    Arc::make_mut(&mut encryption_info).verification_state =
        VerificationState::Unverified(VerificationLevel::UnverifiedIdentity);
    let edit_event = TimelineEvent::from_decrypted(
        DecryptedRoomEvent {
            event: edit_event,
            encryption_info: encryption_info.clone(),
            unsigned_encryption_info: None,
        },
        None,
    );

    timeline.handle_live_event(edit_event).await;

    let items = timeline.controller.items().await;
    let first_event = items[1].as_event().unwrap();

    assert_eq!(
        first_event.encryption_info().unwrap().verification_state,
        VerificationState::Unverified(VerificationLevel::UnverifiedIdentity)
    );

    assert_let!(Some(message) = first_event.content().as_message());
    assert_let!(MessageType::Text(text) = message.msgtype());
    assert_eq!(text.body, "!!edited!! **better** message");
}

#[async_test]
async fn test_relations_edit_overrides_pending_edit_msg() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;

    let original_event_id = event_id!("$original");
    let edit1_event_id = event_id!("$edit1");
    let edit2_event_id = event_id!("$edit2");

    // Pending edit is stashed, nothing comes from the stream.
    timeline
        .handle_live_event(
            f.text_msg("*edit 1")
                .sender(*ALICE)
                .edit(original_event_id, MessageType::text_plain("edit 1").into())
                .event_id(edit1_event_id),
        )
        .await;
    assert_pending!(stream);

    // Now we receive the original event, with a bundled relations group.
    let ev = f.text_msg("original").sender(*ALICE).event_id(original_event_id).with_bundled_edit(
        f.text_msg("* edit 2")
            .edit(original_event_id, MessageType::text_plain("edit 2").into())
            .event_id(edit2_event_id)
            .sender(*ALICE),
    );

    timeline.handle_live_event(ev).await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);

    // We receive the latest edit, not the pending one.
    let event = item.as_event().unwrap();
    assert_eq!(
        event
            .latest_edit_json()
            .expect("we should have an edit json")
            .deserialize()
            .unwrap()
            .event_id(),
        edit2_event_id
    );

    let text = event.content().as_message().unwrap();
    assert_eq!(text.body(), "edit 2");

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());

    assert_pending!(stream);
}

#[async_test]
async fn test_chained_local_edits_resolve_to_the_most_recent_one() {
    // Several edits of the same event pending at once: the timeline must show the
    // last one, not the first one recorded.
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;

    let original_event_id = event_id!("$original");

    timeline
        .handle_live_event(f.text_msg("original").sender(*ALICE).event_id(original_event_id))
        .await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    assert_eq!(item.as_event().unwrap().content().as_message().unwrap().body(), "original");

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());

    let local_edit = |body: &str| {
        let mut content = RoomMessageEventContent::text_plain(format!("* {body}"));
        content.relates_to = Some(Relation::Replacement(Replacement::new(
            original_event_id.to_owned(),
            RoomMessageEventContentWithoutRelation::text_plain(body),
        )));
        AnyMessageLikeEventContent::RoomMessage(content)
    };

    timeline.handle_local_event(local_edit("edit 1")).await;

    let item = assert_next_matches!(stream, VectorDiff::Set { value, .. } => value);
    assert_eq!(item.as_event().unwrap().content().as_message().unwrap().body(), "edit 1");

    // A second edit, sent before the first one has reached the server, must win.
    timeline.handle_local_event(local_edit("edit 2")).await;

    let item = assert_next_matches!(stream, VectorDiff::Set { value, .. } => value);
    assert_eq!(item.as_event().unwrap().content().as_message().unwrap().body(), "edit 2");

    assert_pending!(stream);
}

#[async_test]
async fn test_chained_bundled_edits_resolve_to_the_most_recent_one() {
    // Two edits of the same event, neither of which has its own event in the
    // loaded window, so their positions can't order them: the newest by
    // `origin_server_ts` wins, per MSC2676.
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;

    let original_event_id = event_id!("$original");
    let edit1_event_id = event_id!("$edit1");
    let edit2_event_id = event_id!("$edit2");

    // The *newest* edit is bundled with the original event.
    timeline
        .handle_live_event(
            f.text_msg("original")
                .sender(*ALICE)
                .event_id(original_event_id)
                .server_ts(1000)
                .with_bundled_edit(
                    f.text_msg("* edit 2")
                        .sender(*ALICE)
                        .edit(original_event_id, MessageType::text_plain("edit 2").into())
                        .event_id(edit2_event_id)
                        .server_ts(3000),
                ),
        )
        .await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let event = item.as_event().unwrap();
    assert_eq!(event.content().as_message().unwrap().body(), "edit 2");
    assert_eq!(event.latest_edit_json().unwrap().deserialize().unwrap().event_id(), edit2_event_id);

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());

    // An older edit arrives afterwards, also without its own event in the window.
    // It must not override the newer one just because it was recorded last.
    timeline
        .handle_live_event(
            f.text_msg("hi")
                .sender(*ALICE)
                .event_id(event_id!("$other"))
                .server_ts(1500)
                .with_bundled_edit(
                    f.text_msg("* edit 1")
                        .sender(*ALICE)
                        .edit(original_event_id, MessageType::text_plain("edit 1").into())
                        .event_id(edit1_event_id)
                        .server_ts(2000),
                ),
        )
        .await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    assert_eq!(item.as_event().unwrap().content().as_message().unwrap().body(), "hi");

    let items = timeline.controller.items().await;
    let edited = items
        .iter()
        .filter_map(|item| item.as_event())
        .find(|event| event.event_id() == Some(original_event_id))
        .unwrap();
    assert_eq!(edited.content().as_message().unwrap().body(), "edit 2");
    assert_eq!(
        edited.latest_edit_json().unwrap().deserialize().unwrap().event_id(),
        edit2_event_id
    );

    assert_pending!(stream);
}

#[async_test]
async fn test_relations_edit_overrides_pending_edit_poll() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe().await;

    let f = &timeline.factory;

    let original_event_id = event_id!("$original");
    let edit1_event_id = event_id!("$edit1");
    let edit2_event_id = event_id!("$edit2");

    // Pending edit is stashed, nothing comes from the stream.
    timeline
        .handle_live_event(
            f.poll_edit(
                original_event_id,
                "Can the fake slim shady please stand up?",
                vec!["Excuse me?"],
            )
            .sender(*ALICE)
            .event_id(edit1_event_id),
        )
        .await;
    assert_pending!(stream);

    // Now we receive the original event, with a bundled relations group.
    let ev = f
        .poll_start(
            "Can the fake slim shady please stand down?\nExcuse me?",
            "Can the fake slim shady please stand down?",
            vec!["Excuse me?"],
        )
        .sender(*ALICE)
        .event_id(original_event_id)
        .with_bundled_edit(
            f.poll_edit(
                original_event_id,
                "Can the real slim shady please stand up?",
                vec!["Excuse me?", "Please stand up 🎵", "Please stand up 🎶"],
            )
            .sender(*ALICE)
            .event_id(edit2_event_id),
        );

    timeline.handle_live_event(ev).await;

    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);

    // We receive the latest edit, not the pending one.
    let event = item.as_event().unwrap();
    assert_eq!(
        event
            .latest_edit_json()
            .expect("we should have an edit json")
            .deserialize()
            .unwrap()
            .event_id(),
        edit2_event_id
    );

    let poll = event.content().as_poll().unwrap();
    assert!(poll.has_been_edited);
    assert_eq!(poll.poll_start.question.text, "Can the real slim shady please stand up?");
    assert_eq!(poll.poll_start.answers.len(), 3);

    let date_divider = assert_next_matches!(stream, VectorDiff::PushFront { value } => value);
    assert!(date_divider.is_date_divider());

    assert_pending!(stream);
}

#[async_test]
async fn test_updated_reply_doesnt_lose_latest_edit() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe_events().await;

    let f = &timeline.factory;

    // Start with a message event.
    let target = event_id!("$1");
    timeline.handle_live_event(f.text_msg("hey").sender(&ALICE).event_id(target)).await;

    {
        let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
        assert!(item.latest_edit_json().is_none());
        assert_eq!(item.content().as_message().unwrap().body(), "hey");
        assert_pending!(stream);
    }

    // Have someone send a reply.
    let reply = event_id!("$2");
    timeline
        .handle_live_event(f.text_msg("hallo").sender(&BOB).reply_to(target).event_id(reply))
        .await;

    {
        let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
        assert!(item.latest_edit_json().is_none());
        assert_eq!(item.content().as_message().unwrap().body(), "hallo");
        assert_pending!(stream);
    }

    // Edit the reply.
    timeline
        .handle_live_event(
            f.text_msg("* guten tag")
                .sender(&BOB)
                .edit(reply, MessageType::text_plain("guten tag").into()),
        )
        .await;

    {
        let item = assert_next_matches!(stream, VectorDiff::Set { index: 1, value } => value);
        assert!(item.latest_edit_json().is_some());
        assert_eq!(item.content().as_message().unwrap().body(), "guten tag");
        assert_pending!(stream);
    }

    // Edit the original.
    timeline
        .handle_live_event(
            f.text_msg("* hello")
                .sender(&ALICE)
                .edit(target, MessageType::text_plain("hello").into()),
        )
        .await;

    // The original is updated.
    let item = assert_next_matches!(stream, VectorDiff::Set { index: 0, value } => value);
    // And now has a latest edit JSON.
    assert!(item.latest_edit_json().is_some());

    // The reply is updated.
    let item = assert_next_matches!(stream, VectorDiff::Set { index: 1, value } => value);
    // And still has the latest edit JSON.
    assert!(item.latest_edit_json().is_some());
    assert_eq!(item.content().as_message().unwrap().body(), "guten tag");

    assert_pending!(stream);
}

#[async_test]
async fn test_failed_edit_is_reported_on_the_item_it_edits() {
    // An edit has no timeline item of its own: without this, a failed edit looks
    // exactly like a sent one, and there is no way to retry it.
    let timeline = TestTimeline::new().await;

    let f = &timeline.factory;
    timeline.handle_live_event(f.text_msg("hello").sender(&ALICE)).await;

    let event_id =
        timeline.controller.items().await[1].as_event().unwrap().event_id().unwrap().to_owned();

    // Queue an edit of it: it applies right away, and reports that it hasn't been
    // sent yet.
    let txn_id = timeline
        .handle_local_event(AnyMessageLikeEventContent::RoomMessage(
            RoomMessageEventContentWithoutRelation::text_plain("hi")
                .make_replacement(ReplacementMetadata::new(event_id.clone(), None)),
        ))
        .await;

    let items = timeline.controller.items().await;
    let item = items[1].as_event().unwrap();
    assert_let!(Some(message) = item.content().as_message());
    assert_eq!(message.body(), "hi");
    assert_matches!(
        item.local_edit().map(|edit| &edit.send_state),
        Some(EventSendState::NotSentYet { .. })
    );

    // The edit fails to be sent: the item says so.
    timeline
        .handle_room_send_queue_update(RoomSendQueueUpdate::SendError {
            transaction_id: txn_id.clone(),
            error: Arc::new(matrix_sdk::Error::SendQueueWedgeError(Box::new(
                QueueWedgeError::GenericApiError { msg: "nope".to_owned() },
            ))),
            is_recoverable: false,
        })
        .await;

    let items = timeline.controller.items().await;
    let item = items[1].as_event().unwrap();
    // The edit stays applied, so the user doesn't lose what they typed.
    assert_eq!(item.content().as_message().unwrap().body(), "hi");
    assert_matches!(
        item.local_edit().map(|edit| &edit.send_state),
        Some(EventSendState::SendingFailed { is_recoverable: false, .. })
    );

    // Once the edit is sent, the item stops reporting it.
    timeline
        .handle_room_send_queue_update(RoomSendQueueUpdate::SentEvent {
            transaction_id: txn_id,
            event_id: event_id!("$edit").to_owned(),
        })
        .await;

    let items = timeline.controller.items().await;
    let item = items[1].as_event().unwrap();
    assert_eq!(item.content().as_message().unwrap().body(), "hi");
    assert!(item.local_edit().is_none());
}

#[async_test]
async fn test_edit_arriving_before_its_target_is_not_dropped() {
    // An edit whose target we haven't seen yet is stashed, and applied when the
    // target eventually arrives.
    let timeline = TestTimeline::new().await;

    let f = &timeline.factory;
    let target_id = event_id!("$original");

    timeline
        .handle_live_event(
            f.text_msg("* edited")
                .edit(target_id, RoomMessageEventContentWithoutRelation::text_plain("edited"))
                .sender(&ALICE)
                .event_id(event_id!("$edit")),
        )
        .await;

    // The edit alone produces no item of its own.
    assert!(timeline.controller.items().await.is_empty());

    timeline.handle_live_event(f.text_msg("original").sender(&ALICE).event_id(target_id)).await;

    let items = timeline.controller.items().await;
    assert_eq!(items.len(), 2);
    assert_let!(Some(message) = items[1].as_event().unwrap().content().as_message());
    assert_eq!(message.body(), "edited");
    assert!(message.is_edited());
}

#[async_test]
async fn test_a_later_copy_of_the_original_doesnt_revert_an_edit() {
    // The same event coming down again - a duplicate from a gappy sync, a
    // re-decryption - must not take the item back to its unedited content.
    let timeline = TestTimeline::new().await;

    let f = &timeline.factory;
    let target_id = event_id!("$original");

    timeline.handle_live_event(f.text_msg("original").sender(&ALICE).event_id(target_id)).await;
    timeline
        .handle_live_event(
            f.text_msg("* edited")
                .edit(target_id, RoomMessageEventContentWithoutRelation::text_plain("edited"))
                .sender(&ALICE)
                .event_id(event_id!("$edit")),
        )
        .await;

    let items = timeline.controller.items().await;
    assert_eq!(items[1].as_event().unwrap().content().as_message().unwrap().body(), "edited");

    // Here comes the unedited original again.
    timeline.handle_live_event(f.text_msg("original").sender(&ALICE).event_id(target_id)).await;

    let items = timeline.controller.items().await;
    let message = items
        .iter()
        .filter_map(|item| item.as_event())
        .last()
        .and_then(|event| event.content().as_message())
        .expect("the edited message is still there");
    assert_eq!(message.body(), "edited");
}

#[async_test]
async fn test_redacting_an_edit_restores_the_original_content() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe_events().await;

    let f = &timeline.factory;
    let edit_event_id = event_id!("$edit");

    timeline.handle_live_event(f.text_msg("original").sender(&ALICE)).await;
    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let original_event_id = item.event_id().unwrap().to_owned();
    assert!(!item.content().as_message().unwrap().is_edited());

    // The message is edited.
    timeline
        .handle_live_event(
            f.text_msg(" * edited")
                .sender(&ALICE)
                .event_id(edit_event_id)
                .edit(&original_event_id, MessageType::text_plain("edited").into()),
        )
        .await;

    let item = assert_next_matches!(stream, VectorDiff::Set { value, .. } => value);
    let message = item.content().as_message().unwrap();
    assert_eq!(message.body(), "edited");
    assert!(message.is_edited());
    assert!(item.latest_edit_json().is_some());

    // Then the edit event itself is redacted: the message goes back to what it
    // was before the edit.
    timeline.handle_live_event(f.redaction(edit_event_id).sender(&ALICE)).await;

    let item = assert_next_matches!(stream, VectorDiff::Set { value, .. } => value);
    let message = item.content().as_message().unwrap();
    assert_eq!(message.body(), "original");
    assert!(!message.is_edited());
    assert!(item.latest_edit_json().is_none());
}

#[async_test]
async fn test_cancelling_the_local_echo_of_an_edit_restores_the_original_content() {
    let timeline = TestTimeline::new().await;
    let mut stream = timeline.subscribe_events().await;

    let f = &timeline.factory;

    // ALICE is the timeline's own user, so this is a message we can edit.
    timeline.handle_live_event(f.text_msg("original").sender(&ALICE)).await;
    let item = assert_next_matches!(stream, VectorDiff::PushBack { value } => value);
    let original_event_id = item.event_id().unwrap().to_owned();
    assert!(!item.content().as_message().unwrap().is_edited());

    // Send a local echo of an edit of that message.
    let edit = RoomMessageEventContent::text_plain("edited")
        .make_replacement(ReplacementMetadata::new(original_event_id.clone(), None));
    let txn_id = timeline.handle_local_event(edit.into()).await;

    // The edit is applied in place, it doesn't get an item of its own.
    let item = assert_next_matches!(stream, VectorDiff::Set { value, .. } => value);
    let message = item.content().as_message().unwrap();
    assert_eq!(message.body(), "edited");
    assert!(message.is_edited());

    // Now the edit is cancelled before it is ever sent: the message must go back
    // to what it was, rather than being stuck showing the edited content.
    timeline
        .handle_room_send_queue_update(RoomSendQueueUpdate::CancelledLocalEvent {
            transaction_id: txn_id,
        })
        .await;

    let item = assert_next_matches!(stream, VectorDiff::Set { value, .. } => value);
    let message = item.content().as_message().unwrap();
    assert_eq!(message.body(), "original");
    assert!(!message.is_edited());

    assert_pending!(stream);
}
