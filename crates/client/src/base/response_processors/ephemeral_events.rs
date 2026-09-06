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

use harana_matrix_common::{RoomId, events::AnySyncEphemeralRoomEvent, serde::Raw};
use tracing::info;

use super::Context;

/// Dispatch [`AnySyncEphemeralRoomEvent`]s on the [`Context`].
pub fn dispatch(
    context: &mut Context,
    raw_events: &[Raw<AnySyncEphemeralRoomEvent>],
    room_id: &RoomId,
) {
    for raw_event in raw_events {
        dispatch_receipt(context, raw_event, room_id);
    }
}

/// Dispatch the [`AnySyncEphemeralRoomEvent::Receipt`] on the [`Context`].
pub(super) fn dispatch_receipt(
    context: &mut Context,
    raw_event: &Raw<AnySyncEphemeralRoomEvent>,
    room_id: &RoomId,
) {
    match raw_event.deserialize() {
        Ok(AnySyncEphemeralRoomEvent::Receipt(event)) => {
            context.state_changes.add_receipts(room_id, event.content);
        }

        Ok(_) => {}

        Err(e) => {
            let event_id = raw_event.get_field::<String>("event_id").ok().flatten();

            info!(?room_id, event_id, "Failed to deserialize ephemeral room event: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use harana_matrix_common::{
        event_id,
        events::receipt::{ReceiptThread, ReceiptType},
        room_id,
        serde::Raw,
        user_id,
    };
    use serde_json::json;

    use super::{super::Context, dispatch};
    use crate::test::event_factory::EventFactory;

    /// An `m.receipt` event is turned into receipts attached to the room.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#receipts>.
    #[test]
    fn test_public_read_receipt_is_dispatched() {
        let room_id = room_id!("!r:localhost");
        let f = EventFactory::new();
        let event = f
            .read_receipts()
            .add(
                event_id!("$1"),
                user_id!("@alice:localhost"),
                ReceiptType::Read,
                ReceiptThread::Unthreaded,
            )
            .into_event()
            .into_raw();

        let mut context = Context::default();
        dispatch(&mut context, &[event], room_id);

        let receipts = context.state_changes.receipts.get(room_id).unwrap();
        let by_type = receipts.0.get(event_id!("$1")).unwrap();
        let receipt = by_type.get(&ReceiptType::Read).unwrap().get(user_id!("@alice:localhost"));

        assert!(receipt.is_some());
    }

    /// Private read receipts (`m.read.private`) and threaded receipts are
    /// dispatched too, and are kept apart from the public unthreaded ones.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#receipts> and
    /// <https://spec.matrix.org/v1.16/client-server-api/#threaded-read-receipts>.
    #[test]
    fn test_private_and_threaded_receipts_are_dispatched() {
        let room_id = room_id!("!r:localhost");
        let alice = user_id!("@alice:localhost");
        let f = EventFactory::new();
        let event = f
            .read_receipts()
            .add(event_id!("$1"), alice, ReceiptType::Read, ReceiptThread::Unthreaded)
            .add(event_id!("$2"), alice, ReceiptType::ReadPrivate, ReceiptThread::Unthreaded)
            .add(
                event_id!("$3"),
                alice,
                ReceiptType::Read,
                ReceiptThread::Thread(event_id!("$root").to_owned()),
            )
            .into_event()
            .into_raw();

        let mut context = Context::default();
        dispatch(&mut context, &[event], room_id);

        let receipts = &context.state_changes.receipts.get(room_id).unwrap().0;

        assert!(
            receipts
                .get(event_id!("$1"))
                .unwrap()
                .get(&ReceiptType::Read)
                .unwrap()
                .contains_key(alice)
        );
        assert!(
            receipts
                .get(event_id!("$2"))
                .unwrap()
                .get(&ReceiptType::ReadPrivate)
                .unwrap()
                .contains_key(alice)
        );

        let threaded = receipts
            .get(event_id!("$3"))
            .unwrap()
            .get(&ReceiptType::Read)
            .unwrap()
            .get(alice)
            .unwrap();
        assert_eq!(threaded.thread, ReceiptThread::Thread(event_id!("$root").to_owned()));
    }

    /// Receipts of several users for the same event are all dispatched.
    #[test]
    fn test_receipts_of_several_users_are_dispatched() {
        let room_id = room_id!("!r:localhost");
        let f = EventFactory::new();
        let event = f
            .read_receipts()
            .add(
                event_id!("$1"),
                user_id!("@alice:localhost"),
                ReceiptType::Read,
                ReceiptThread::Unthreaded,
            )
            .add(
                event_id!("$1"),
                user_id!("@bob:localhost"),
                ReceiptType::Read,
                ReceiptThread::Unthreaded,
            )
            .into_event()
            .into_raw();

        let mut context = Context::default();
        dispatch(&mut context, &[event], room_id);

        let by_type =
            context.state_changes.receipts.get(room_id).unwrap().0.get(event_id!("$1")).unwrap();

        assert_eq!(by_type.get(&ReceiptType::Read).unwrap().len(), 2);
    }

    /// The last `m.receipt` event of a batch wins, since a receipt event is
    /// not additive at this level.
    #[test]
    fn test_the_last_receipt_event_wins() {
        let room_id = room_id!("!r:localhost");
        let f = EventFactory::new();
        let first = f
            .read_receipts()
            .add(
                event_id!("$1"),
                user_id!("@alice:localhost"),
                ReceiptType::Read,
                ReceiptThread::Unthreaded,
            )
            .into_event()
            .into_raw();
        let second = f
            .read_receipts()
            .add(
                event_id!("$2"),
                user_id!("@alice:localhost"),
                ReceiptType::Read,
                ReceiptThread::Unthreaded,
            )
            .into_event()
            .into_raw();

        let mut context = Context::default();
        dispatch(&mut context, &[first, second], room_id);

        let receipts = &context.state_changes.receipts.get(room_id).unwrap().0;
        assert!(receipts.get(event_id!("$1")).is_none());
        assert!(receipts.get(event_id!("$2")).is_some());
    }

    /// An `m.typing` event is an ephemeral room event, but it isn't a receipt,
    /// so it must not end up in the receipts.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#typing-notifications>.
    #[test]
    fn test_typing_notification_is_not_a_receipt() {
        let room_id = room_id!("!r:localhost");
        let f = EventFactory::new();
        let event = f.typing(vec![user_id!("@alice:localhost")]).into_raw();

        let mut context = Context::default();
        dispatch(&mut context, &[event], room_id);

        assert!(context.state_changes.receipts.is_empty());
    }

    /// A malformed ephemeral event is skipped, and doesn't prevent the
    /// following events from being dispatched.
    #[test]
    fn test_malformed_event_is_skipped() {
        let room_id = room_id!("!r:localhost");
        let f = EventFactory::new();
        let malformed =
            Raw::new(&json!({ "type": "m.receipt", "content": 42 })).unwrap().cast_unchecked();
        let event = f
            .read_receipts()
            .add(
                event_id!("$1"),
                user_id!("@alice:localhost"),
                ReceiptType::Read,
                ReceiptThread::Unthreaded,
            )
            .into_event()
            .into_raw();

        let mut context = Context::default();
        dispatch(&mut context, &[malformed, event], room_id);

        assert!(context.state_changes.receipts.contains_key(room_id));
    }
}
