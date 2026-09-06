// Copyright 2024 The Matrix.org Foundation C.I.C.
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

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    mem,
};

use ruma::{
    RoomId,
    events::{
        AnyGlobalAccountDataEvent, GlobalAccountDataEventType, direct::OwnedDirectUserIdentifier,
    },
    serde::Raw,
};
use sdk_common::timer;
use tracing::{debug, instrument, trace, warn};

use super::super::Context;
use crate::{RoomInfo, StateChanges, StateStore, store::BaseStateStore};

/// Create the [`Global`] account data processor.
pub fn global(events: &[Raw<AnyGlobalAccountDataEvent>]) -> Global {
    Global::process(events)
}

#[must_use]
pub struct Global {
    parsed_events: Vec<AnyGlobalAccountDataEvent>,
    raw_by_type: BTreeMap<GlobalAccountDataEventType, Raw<AnyGlobalAccountDataEvent>>,
}

impl Global {
    /// Creates a new processor for global account data.
    fn process(events: &[Raw<AnyGlobalAccountDataEvent>]) -> Self {
        let _timer = timer!(tracing::Level::TRACE, "Global::process (global account data)");

        let mut raw_by_type = BTreeMap::new();
        let mut parsed_events = Vec::new();

        for raw_event in events {
            let event = match raw_event.deserialize() {
                Ok(e) => e,
                Err(e) => {
                    let event_type: Option<String> = raw_event.get_field("type").ok().flatten();
                    warn!(event_type, "Failed to deserialize a global account data event: {e}");
                    continue;
                }
            };

            raw_by_type.insert(event.event_type(), raw_event.clone());
            parsed_events.push(event);
        }

        Self { raw_by_type, parsed_events }
    }

    /// Returns the push rules found by this processor.
    pub fn push_rules(&self) -> Option<&Raw<AnyGlobalAccountDataEvent>> {
        self.raw_by_type.get(&GlobalAccountDataEventType::PushRules)
    }

    /// Processes the direct rooms in a sync response:
    ///
    /// Given a [`StateChanges`] instance, processes any direct room info
    /// from the global account data and adds it to the room infos to
    /// save.
    #[instrument(skip_all)]
    fn process_direct_rooms(
        &self,
        events: &[AnyGlobalAccountDataEvent],
        state_store: &BaseStateStore,
        state_changes: &mut StateChanges,
    ) {
        for event in events {
            let AnyGlobalAccountDataEvent::Direct(direct_event) = event else { continue };

            let mut new_dms = HashMap::<&RoomId, HashSet<OwnedDirectUserIdentifier>>::new();

            for (user_identifier, rooms) in direct_event.content.iter() {
                for room_id in rooms {
                    new_dms.entry(room_id).or_default().insert(user_identifier.clone());
                }
            }

            let rooms = state_store.rooms();
            let mut old_dms = rooms
                .iter()
                .filter_map(|r| {
                    let direct_targets = r.direct_targets();
                    (!direct_targets.is_empty()).then(|| (r.room_id(), direct_targets))
                })
                .collect::<HashMap<_, _>>();

            // Update the direct targets of rooms if they changed.
            for (room_id, new_direct_targets) in new_dms {
                if let Some(old_direct_targets) = old_dms.remove(&room_id)
                    && old_direct_targets == new_direct_targets
                {
                    continue;
                }
                trace!(?room_id, targets = ?new_direct_targets, "Marking room as direct room");
                map_info(room_id, state_changes, state_store, |info| {
                    info.base_info.dm_targets = new_direct_targets;
                });
            }

            // Remove the targets of old direct chats.
            for room_id in old_dms.keys() {
                trace!(?room_id, "Unmarking room as direct room");
                map_info(room_id, state_changes, state_store, |info| {
                    info.base_info.dm_targets.clear();
                });
            }
        }
    }

    /// Applies the processed data to the state changes and the state store.
    pub async fn apply(mut self, context: &mut Context, state_store: &BaseStateStore) {
        let _timer = timer!(tracing::Level::TRACE, "Global::apply (global account data)");

        // Fill in the content of `changes.account_data`.
        mem::swap(&mut context.state_changes.account_data, &mut self.raw_by_type);

        // Process direct rooms.
        let has_new_direct_room_data = self
            .parsed_events
            .iter()
            .any(|event| event.event_type() == GlobalAccountDataEventType::Direct);

        if has_new_direct_room_data {
            self.process_direct_rooms(&self.parsed_events, state_store, &mut context.state_changes);
        } else if let Ok(Some(direct_account_data)) =
            state_store.get_account_data_event(GlobalAccountDataEventType::Direct).await
        {
            debug!("Found direct room data in the Store, applying it");
            if let Ok(direct_account_data) = direct_account_data.deserialize() {
                self.process_direct_rooms(
                    &[direct_account_data],
                    state_store,
                    &mut context.state_changes,
                );
            } else {
                warn!("Failed to deserialize direct room account data");
            }
        }
    }
}

/// Applies a function to an existing `RoomInfo` if present in changes, or one
/// loaded from the database.
fn map_info<F: FnOnce(&mut RoomInfo)>(
    room_id: &RoomId,
    changes: &mut StateChanges,
    store: &BaseStateStore,
    f: F,
) {
    if let Some(info) = changes.room_infos.get_mut(room_id) {
        f(info);
    } else if let Some(room) = store.room(room_id) {
        let mut info = room.clone_info();
        f(&mut info);
        changes.add_room(info);
    } else if store.already_logged_missing_room.lock().insert(room_id.to_owned()) {
        debug!(room = %room_id, "couldn't find room in state changes or store");
    }
}

#[cfg(test)]
mod tests {
    use assert_matches2::assert_let;
    use ruma::{
        events::{
            AnyGlobalAccountDataEvent, GlobalAccountDataEventType,
            direct::OwnedDirectUserIdentifier,
        },
        room_id,
        serde::Raw,
        user_id,
    };
    use sdk_test::{async_test, event_factory::EventFactory};
    use serde_json::json;

    use crate::{
        RoomState, StateStore as _, response_processors as processors,
        test_utils::logged_in_base_client,
    };

    /// Applies the given global account data events through the processor and
    /// saves the resulting changes.
    async fn process(client: &crate::BaseClient, events: &[Raw<AnyGlobalAccountDataEvent>]) {
        let mut context = processors::Context::default();

        processors::account_data::global(events).apply(&mut context, &client.state_store).await;

        processors::changes::save_and_apply(
            context,
            &client.state_store,
            &client.state_store_lock().lock().await,
            &client.ignore_user_list_changes,
            None,
        )
        .await
        .unwrap();
    }

    fn direct_event(pairs: &[(&str, &[&str])]) -> Raw<AnyGlobalAccountDataEvent> {
        let f = EventFactory::new();
        let mut builder = f.direct();

        for (user_id, room_ids) in pairs {
            let user_id: OwnedDirectUserIdentifier =
                <&ruma::UserId>::try_from(*user_id).unwrap().into();

            for room_id in *room_ids {
                builder =
                    builder.add_user(user_id.clone(), <&ruma::RoomId>::try_from(*room_id).unwrap());
            }
        }

        builder.into_raw()
    }

    /// The `m.direct` event is stored as-is in the account data of the store.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mdirect>.
    #[async_test]
    async fn test_direct_event_is_stored_in_account_data() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!dm:localhost");

        client.get_or_create_room(room_id, RoomState::Joined);

        process(&client, &[direct_event(&[("@bob:localhost", &["!dm:localhost"])])]).await;

        let stored = client
            .state_store
            .get_account_data_event(GlobalAccountDataEventType::Direct)
            .await
            .unwrap()
            .expect("the m.direct event must have been stored");
        assert_let!(AnyGlobalAccountDataEvent::Direct(stored) = stored.deserialize().unwrap());
        assert_eq!(stored.content.0.len(), 1);
    }

    /// A room listed in `m.direct` becomes a direct room, with the listed user
    /// as its target.
    #[async_test]
    async fn test_room_listed_in_m_direct_becomes_a_dm() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!dm:localhost");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        // Sanity check: the room isn't a DM yet.
        assert!(room.direct_targets().is_empty());

        process(&client, &[direct_event(&[("@bob:localhost", &["!dm:localhost"])])]).await;

        let targets = room.direct_targets();
        assert_eq!(targets.len(), 1);
        assert!(targets.contains(&OwnedDirectUserIdentifier::from(user_id!("@bob:localhost"))));
        assert!(room.is_direct().await.unwrap());
    }

    /// The same room can be listed for several users, in which case all of
    /// them are direct targets of that room.
    #[async_test]
    async fn test_a_room_can_have_several_direct_targets() {
        let client = logged_in_base_client(None).await;
        let room_id = room_id!("!dm:localhost");
        let room = client.get_or_create_room(room_id, RoomState::Joined);

        process(
            &client,
            &[direct_event(&[
                ("@bob:localhost", &["!dm:localhost"]),
                ("@carol:localhost", &["!dm:localhost"]),
            ])],
        )
        .await;

        let targets = room.direct_targets();
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&OwnedDirectUserIdentifier::from(user_id!("@bob:localhost"))));
        assert!(targets.contains(&OwnedDirectUserIdentifier::from(user_id!("@carol:localhost"))));
    }

    /// A user can have several direct rooms; each of them must be marked as a
    /// DM.
    #[async_test]
    async fn test_a_user_can_have_several_direct_rooms() {
        let client = logged_in_base_client(None).await;
        let first = client.get_or_create_room(room_id!("!dm1:localhost"), RoomState::Joined);
        let second = client.get_or_create_room(room_id!("!dm2:localhost"), RoomState::Joined);

        process(
            &client,
            &[direct_event(&[("@bob:localhost", &["!dm1:localhost", "!dm2:localhost"])])],
        )
        .await;

        assert_eq!(first.direct_targets_length(), 1);
        assert_eq!(second.direct_targets_length(), 1);
    }

    /// When a room is removed from the `m.direct` event, it stops being a
    /// direct room.
    #[async_test]
    async fn test_room_removed_from_m_direct_is_not_a_dm_anymore() {
        let client = logged_in_base_client(None).await;
        let room = client.get_or_create_room(room_id!("!dm:localhost"), RoomState::Joined);

        process(&client, &[direct_event(&[("@bob:localhost", &["!dm:localhost"])])]).await;
        assert_eq!(room.direct_targets_length(), 1);

        // The user now has a different direct room; the previous one isn't
        // direct anymore.
        process(&client, &[direct_event(&[("@bob:localhost", &["!other:localhost"])])]).await;

        assert!(room.direct_targets().is_empty());
        assert!(!room.is_direct().await.unwrap());
    }

    /// When a sync doesn't carry an `m.direct` event, the one that is already
    /// in the store is applied, so that rooms discovered later are still
    /// marked as direct.
    #[async_test]
    async fn test_stored_m_direct_is_applied_to_newly_known_rooms() {
        let client = logged_in_base_client(None).await;

        // The `m.direct` event arrives before the room is known.
        process(&client, &[direct_event(&[("@bob:localhost", &["!dm:localhost"])])]).await;

        let room = client.get_or_create_room(room_id!("!dm:localhost"), RoomState::Joined);
        assert!(room.direct_targets().is_empty());

        // A later sync without any global account data still applies the
        // stored `m.direct` event.
        process(&client, &[]).await;

        assert_eq!(room.direct_targets_length(), 1);
    }

    /// A malformed global account data event must be skipped, and must not
    /// prevent the other events from being processed.
    #[async_test]
    async fn test_malformed_event_is_skipped() {
        let client = logged_in_base_client(None).await;
        let room = client.get_or_create_room(room_id!("!dm:localhost"), RoomState::Joined);

        let malformed = Raw::new(&json!({ "type": 42 })).unwrap().cast_unchecked();

        process(&client, &[malformed, direct_event(&[("@bob:localhost", &["!dm:localhost"])])])
            .await;

        assert_eq!(room.direct_targets_length(), 1);
    }

    /// The processor exposes the `m.push_rules` event it has seen, so that the
    /// push rules can be updated.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#push-rules>.
    #[async_test]
    async fn test_push_rules_are_exposed() {
        // No `m.push_rules` event in the batch.
        let processor = processors::account_data::global(&[direct_event(&[])]);
        assert!(processor.push_rules().is_none());

        let push_rules: Raw<AnyGlobalAccountDataEvent> = Raw::new(&json!({
            "type": "m.push_rules",
            "content": {
                "global": {
                    "content": [],
                    "override": [],
                    "room": [],
                    "sender": [],
                    "underride": [],
                },
            },
        }))
        .unwrap()
        .cast_unchecked();

        let processor = processors::account_data::global(&[push_rules]);
        assert!(processor.push_rules().is_some());
    }
}
