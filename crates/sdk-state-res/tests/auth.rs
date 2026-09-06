// Copyright 2026 The Harana Contributors
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

//! Authorization against an asynchronous store.

use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use ruma::{
    EventId, MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedRoomId, OwnedUserId, RoomId, UserId,
    events::{StateEventType, TimelineEventType},
    room_version_rules::RoomVersionRules,
    uint,
};
use sdk_state_res::{AuthCheckOutcome, Event, auth_check, check_state_dependent_auth_rules};
use sdk_test_macros::async_test;
use serde_json::{Value as JsonValue, json, value::RawValue as RawJsonValue};

/// The smallest event a state resolution consumer can hold.
#[derive(Clone, Debug)]
struct TestPdu {
    event_id: OwnedEventId,
    room_id: OwnedRoomId,
    sender: OwnedUserId,
    event_type: TimelineEventType,
    state_key: Option<String>,
    content: Box<RawJsonValue>,
    prev_events: Vec<OwnedEventId>,
    auth_events: Vec<OwnedEventId>,
}

impl Event for TestPdu {
    type Id = OwnedEventId;

    fn event_id(&self) -> &Self::Id {
        &self.event_id
    }

    fn room_id(&self) -> Option<&RoomId> {
        Some(&self.room_id)
    }

    fn sender(&self) -> &UserId {
        &self.sender
    }

    fn origin_server_ts(&self) -> MilliSecondsSinceUnixEpoch {
        MilliSecondsSinceUnixEpoch(uint!(0))
    }

    fn event_type(&self) -> &TimelineEventType {
        &self.event_type
    }

    fn content(&self) -> &RawJsonValue {
        &self.content
    }

    fn state_key(&self) -> Option<&str> {
        self.state_key.as_deref()
    }

    fn prev_events(&self) -> Box<dyn DoubleEndedIterator<Item = &Self::Id> + '_> {
        Box::new(self.prev_events.iter())
    }

    fn auth_events(&self) -> Box<dyn DoubleEndedIterator<Item = &Self::Id> + '_> {
        Box::new(self.auth_events.iter())
    }

    fn redacts(&self) -> Option<&Self::Id> {
        None
    }

    fn rejected(&self) -> bool {
        false
    }
}

const ROOM_ID: &str = "!room:localhost";
const ALICE: &str = "@alice:localhost";
const BOB: &str = "@bob:localhost";

fn event(
    id: &str,
    sender: &str,
    event_type: TimelineEventType,
    state_key: Option<&str>,
    content: JsonValue,
    auth_events: &[&str],
) -> TestPdu {
    TestPdu {
        event_id: EventId::parse(id).unwrap(),
        room_id: RoomId::parse(ROOM_ID).unwrap(),
        sender: UserId::parse(sender).unwrap(),
        event_type,
        state_key: state_key.map(ToOwned::to_owned),
        content: serde_json::value::to_raw_value(&content).unwrap(),
        prev_events: Vec::new(),
        auth_events: auth_events.iter().map(|id| EventId::parse(id).unwrap()).collect(),
    }
}

/// A room with a create event, a joined Alice, and default power levels.
struct Room {
    events: HashMap<OwnedEventId, TestPdu>,
    state: HashMap<(StateEventType, String), TestPdu>,
    /// Every state key the checks looked up, in order.
    // A `Mutex` rather than a `RefCell`: the futures handed to `auth_check`
    // have to be `Send`, and a `RefCell` is not `Sync`.
    state_reads: Mutex<Vec<(StateEventType, String)>>,
    event_reads: AtomicUsize,
}

impl Room {
    fn new() -> Self {
        let create = event(
            "$create:localhost",
            ALICE,
            TimelineEventType::RoomCreate,
            Some(""),
            json!({ "creator": ALICE, "room_version": "11" }),
            &[],
        );
        let alice_member = event(
            "$alice_member:localhost",
            ALICE,
            TimelineEventType::RoomMember,
            Some(ALICE),
            json!({ "membership": "join" }),
            &["$create:localhost"],
        );
        let power_levels = event(
            "$power_levels:localhost",
            ALICE,
            TimelineEventType::RoomPowerLevels,
            Some(""),
            json!({ "users": { ALICE: 100 }, "users_default": 0, "events_default": 0 }),
            &["$create:localhost", "$alice_member:localhost"],
        );

        let mut events = HashMap::new();
        let mut state = HashMap::new();

        for (event, state_event_type) in [
            (create, StateEventType::RoomCreate),
            (alice_member, StateEventType::RoomMember),
            (power_levels, StateEventType::RoomPowerLevels),
        ] {
            state.insert((state_event_type, event.state_key.clone().unwrap()), event.clone());
            events.insert(event.event_id.clone(), event);
        }

        Self {
            events,
            state,
            state_reads: Mutex::new(Vec::new()),
            event_reads: AtomicUsize::new(0),
        }
    }

    // The fetchers stand in for an async store, so they are async even though
    // these read from a map.
    #[allow(clippy::unused_async)]
    async fn fetch_event(&self, event_id: OwnedEventId) -> Option<TestPdu> {
        self.event_reads.fetch_add(1, Ordering::Relaxed);
        self.events.get(&event_id).cloned()
    }

    #[allow(clippy::unused_async)]
    async fn fetch_state(&self, event_type: StateEventType, state_key: String) -> Option<TestPdu> {
        self.state_reads.lock().unwrap().push((event_type.clone(), state_key.clone()));
        self.state.get(&(event_type, state_key)).cloned()
    }
}

fn message(sender: &str, auth_events: &[&str]) -> TestPdu {
    event(
        "$message:localhost",
        sender,
        TimelineEventType::RoomMessage,
        None,
        json!({ "msgtype": "m.text", "body": "hello" }),
        auth_events,
    )
}

#[async_test]
async fn test_a_joined_member_may_send_a_message() {
    let room = Room::new();
    let message = message(
        ALICE,
        &["$create:localhost", "$alice_member:localhost", "$power_levels:localhost"],
    );

    let outcome = auth_check(
        &RoomVersionRules::V11,
        &message,
        |event_id| room.fetch_event(event_id),
        |event_type, state_key| room.fetch_state(event_type, state_key),
    )
    .await
    .unwrap();

    assert_eq!(outcome, AuthCheckOutcome::Allow);
    assert!(outcome.is_allowed());
}

#[async_test]
async fn test_a_user_who_is_not_joined_may_not_send_a_message() {
    let room = Room::new();
    // Bob has no membership event, so the state-dependent rules reject him.
    let message = message(BOB, &["$create:localhost", "$power_levels:localhost"]);

    let outcome = check_state_dependent_auth_rules(
        &RoomVersionRules::V11,
        &message,
        |event_type, state_key| room.fetch_state(event_type, state_key),
    )
    .await
    .unwrap();

    assert!(matches!(outcome, AuthCheckOutcome::Deny(_)), "{outcome:?} should be a denial");
}

#[async_test]
async fn test_missing_state_is_fetched_once_rather_than_re_requested() {
    let room = Room::new();
    let message = message(
        ALICE,
        &["$create:localhost", "$alice_member:localhost", "$power_levels:localhost"],
    );

    check_state_dependent_auth_rules(&RoomVersionRules::V11, &message, |event_type, state_key| {
        room.fetch_state(event_type, state_key)
    })
    .await
    .unwrap();

    // A key the room does not hold is an answer, not a miss, so no key is read
    // twice however many rounds the check takes.
    let reads = room.state_reads.lock().unwrap();
    let mut seen = reads.clone();
    seen.sort();
    seen.dedup();

    assert_eq!(reads.len(), seen.len(), "a state key was read more than once: {reads:?}");
}

#[async_test]
async fn test_auth_events_are_seeded_from_the_event_itself() {
    let room = Room::new();
    let message = message(
        ALICE,
        &["$create:localhost", "$alice_member:localhost", "$power_levels:localhost"],
    );

    auth_check(
        &RoomVersionRules::V11,
        &message,
        |event_id| room.fetch_event(event_id),
        |event_type, state_key| room.fetch_state(event_type, state_key),
    )
    .await
    .unwrap();

    // The three auth events are fetched, and the seeding means no further round
    // of event fetching is needed.
    assert_eq!(room.event_reads.load(Ordering::Relaxed), 3);
}
