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

//! State resolution against an asynchronous store.

use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use harana_matrix_common::{
    EventId, MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedRoomId, OwnedUserId, RoomId, UInt,
    UserId,
    events::{StateEventType, TimelineEventType},
    room_version_rules::RoomVersionRules,
};
use harana_matrix_macros::async_test;
use harana_matrix_server::state_res::{
    Error, Event, StateMap, resolve, utils::event_id_set::EventIdSet,
};
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
    origin_server_ts: UInt,
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
        MilliSecondsSinceUnixEpoch(self.origin_server_ts)
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

const CREATE: &str = "$create:localhost";
const ALICE_MEMBER: &str = "$alice_member:localhost";
const POWER_LEVELS: &str = "$power_levels:localhost";
const TOPIC_ONE: &str = "$topic_one:localhost";
const TOPIC_TWO: &str = "$topic_two:localhost";

#[allow(clippy::too_many_arguments)]
fn event(
    id: &str,
    event_type: TimelineEventType,
    state_key: Option<&str>,
    content: JsonValue,
    prev_events: &[&str],
    auth_events: &[&str],
    origin_server_ts: u64,
) -> TestPdu {
    TestPdu {
        event_id: EventId::parse(id).unwrap(),
        room_id: RoomId::parse(ROOM_ID).unwrap(),
        sender: UserId::parse(ALICE).unwrap(),
        event_type,
        state_key: state_key.map(ToOwned::to_owned),
        content: serde_json::value::to_raw_value(&content).unwrap(),
        prev_events: prev_events.iter().map(|id| EventId::parse(id).unwrap()).collect(),
        auth_events: auth_events.iter().map(|id| EventId::parse(id).unwrap()).collect(),
        origin_server_ts: origin_server_ts.try_into().unwrap(),
    }
}

fn id(event_id: &str) -> OwnedEventId {
    EventId::parse(event_id).unwrap()
}

/// A room whose state forks into two competing topics.
struct Room {
    events: HashMap<OwnedEventId, TestPdu>,
    reads: AtomicUsize,
}

impl Room {
    fn new() -> Self {
        let events = [
            event(
                CREATE,
                TimelineEventType::RoomCreate,
                Some(""),
                json!({ "creator": ALICE, "room_version": "11" }),
                &[],
                &[],
                1,
            ),
            event(
                ALICE_MEMBER,
                TimelineEventType::RoomMember,
                Some(ALICE),
                json!({ "membership": "join" }),
                &[CREATE],
                &[CREATE],
                2,
            ),
            event(
                POWER_LEVELS,
                TimelineEventType::RoomPowerLevels,
                Some(""),
                json!({ "users": { ALICE: 100 }, "users_default": 0, "state_default": 50 }),
                &[ALICE_MEMBER],
                &[CREATE, ALICE_MEMBER],
                3,
            ),
            event(
                TOPIC_ONE,
                TimelineEventType::RoomTopic,
                Some(""),
                json!({ "topic": "one" }),
                &[POWER_LEVELS],
                &[CREATE, ALICE_MEMBER, POWER_LEVELS],
                4,
            ),
            event(
                TOPIC_TWO,
                TimelineEventType::RoomTopic,
                Some(""),
                json!({ "topic": "two" }),
                &[POWER_LEVELS],
                &[CREATE, ALICE_MEMBER, POWER_LEVELS],
                5,
            ),
        ];

        Self {
            events: events.into_iter().map(|event| (event.event_id.clone(), event)).collect(),
            reads: AtomicUsize::new(0),
        }
    }

    // The fetchers stand in for an async store, so they are async even though
    // these read from a map.
    #[allow(clippy::unused_async)]
    async fn fetch_event(&self, event_id: OwnedEventId) -> Option<TestPdu> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.events.get(&event_id).cloned()
    }
}

/// The two forks, each carrying the same base state and its own topic.
fn state_maps() -> Vec<StateMap<OwnedEventId>> {
    let base = [
        ((StateEventType::RoomCreate, String::new()), id(CREATE)),
        ((StateEventType::RoomMember, ALICE.to_owned()), id(ALICE_MEMBER)),
        ((StateEventType::RoomPowerLevels, String::new()), id(POWER_LEVELS)),
    ];

    [TOPIC_ONE, TOPIC_TWO]
        .into_iter()
        .map(|topic| {
            let mut state: StateMap<OwnedEventId> = base.iter().cloned().collect();
            state.insert((StateEventType::RoomTopic, String::new()), id(topic));
            state
        })
        .collect()
}

fn auth_chains() -> Vec<EventIdSet<OwnedEventId>> {
    [TOPIC_ONE, TOPIC_TWO]
        .into_iter()
        .map(|topic| EventIdSet::from([id(CREATE), id(ALICE_MEMBER), id(POWER_LEVELS), id(topic)]))
        .collect()
}

#[async_test]
async fn test_a_conflicted_topic_resolves_to_one_of_the_two() {
    let room = Room::new();
    let state_maps = state_maps();

    let resolved = resolve(
        &RoomVersionRules::V11,
        &state_maps,
        auth_chains(),
        |event_id| room.fetch_event(event_id),
        |_| None,
    )
    .await
    .unwrap();

    // The unconflicted entries survive untouched.
    assert_eq!(resolved.get(&(StateEventType::RoomCreate, String::new())), Some(&id(CREATE)));
    assert_eq!(
        resolved.get(&(StateEventType::RoomPowerLevels, String::new())),
        Some(&id(POWER_LEVELS))
    );

    // The conflicted one resolves to exactly one of the candidates.
    let topic = resolved
        .get(&(StateEventType::RoomTopic, String::new()))
        .expect("the resolved state has a topic");
    assert!([id(TOPIC_ONE), id(TOPIC_TWO)].contains(topic), "unexpected topic {topic}");

    // Every event named by the forks was fetched, and the seeding means each was
    // fetched once.
    assert_eq!(room.reads.load(Ordering::Relaxed), 5);
}

#[async_test]
async fn test_room_version_1_state_resolution_is_reported_as_unsupported() {
    let room = Room::new();
    let state_maps = state_maps();

    let error = resolve(
        &RoomVersionRules::V1,
        &state_maps,
        auth_chains(),
        |event_id| room.fetch_event(event_id),
        |_| None,
    )
    .await
    .unwrap_err();

    assert!(
        matches!(error, Error::UnsupportedStateResolutionVersion),
        "{error:?} should report the unsupported algorithm"
    );
}
