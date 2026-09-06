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

//! Event authorization against an asynchronous store.

use std::{borrow::Borrow, future::Future};

use common_ruma::{
    EventId, OwnedEventId, RoomId,
    events::StateEventType,
    room_version_rules::RoomVersionRules,
    state_res::{Event, auth_types_for_event},
};
use tracing::{debug, instrument};

use crate::{
    Error,
    fetch::{FetchCache, MAX_FETCH_ROUNDS},
};

/// The result of an authorization check that ran to completion.
///
/// A denial is an outcome, not an error: the check ran, and the rules rejected
/// the event. Only a check that could not be run at all is reported as an
/// error, which here means the store never produced an event the rules needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthCheckOutcome {
    /// Authorization accepted the event.
    ///
    /// The caller may continue processing it.
    Allow,

    /// Authorization rejected the event.
    ///
    /// The string is the failing rule's description, as reported by Ruma.
    Deny(String),
}

impl AuthCheckOutcome {
    /// Whether the event was accepted.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Converts the outcome to a result, with the denial reason as the error.
    ///
    /// # Errors
    ///
    /// Returns the denial reason when the outcome is [`Self::Deny`].
    pub fn into_result(self) -> Result<(), String> {
        match self {
            Self::Allow => Ok(()),
            Self::Deny(reason) => Err(reason),
        }
    }
}

/// Checks an event against both the state-independent and the state-dependent
/// authorization rules.
///
/// This is the pair of checks a caller runs on receipt of an event, in the
/// order the specification gives them: the state-independent rules examine the
/// event's own `auth_events`, and the state-dependent rules examine it against
/// a state snapshot. A denial from either is returned as-is.
///
/// `fetch_event` resolves an event ID, and `fetch_state` resolves a state key
/// against whichever snapshot the caller is checking against. Both return
/// `None` for something that does not exist, which is a normal answer and not
/// an error.
///
/// # Errors
///
/// Returns an error when a lookup the rules depend on could not be resolved
/// within [`MAX_FETCH_ROUNDS`] rounds of fetching.
#[instrument(level = "debug", skip_all, fields(event_id = %incoming_event.event_id().borrow()))]
pub async fn auth_check<E, FetchEvent, EventFut, FetchState, StateFut>(
    rules: &RoomVersionRules,
    incoming_event: &E,
    fetch_event: FetchEvent,
    fetch_state: FetchState,
) -> Result<AuthCheckOutcome, Error>
where
    // The lookups are held across the awaits below, so the whole future is
    // only `Send` when they are: these checks are driven from spawned tasks.
    E: Event + Clone + Send + Sync,
    FetchEvent: Fn(OwnedEventId) -> EventFut + Send,
    EventFut: Future<Output = Option<E>> + Send,
    FetchState: Fn(StateEventType, String) -> StateFut + Send,
    StateFut: Future<Output = Option<E>> + Send,
{
    match check_state_independent_auth_rules(rules, incoming_event, fetch_event).await? {
        AuthCheckOutcome::Allow => {}
        deny => return Ok(deny),
    }

    check_state_dependent_auth_rules(rules, incoming_event, fetch_state).await
}

/// Checks an event against the state-independent authorization rules.
///
/// These rules examine the event's own `auth_events`, so the lookups are seeded
/// with exactly those, plus the `m.room.create` event derived from the room ID
/// in room versions that identify it that way.
///
/// # Errors
///
/// Returns an error when a lookup the rules depend on could not be resolved
/// within [`MAX_FETCH_ROUNDS`] rounds of fetching.
#[instrument(level = "debug", skip_all, fields(event_id = %incoming_event.event_id().borrow()))]
pub async fn check_state_independent_auth_rules<E, FetchEvent, EventFut>(
    rules: &RoomVersionRules,
    incoming_event: &E,
    fetch_event: FetchEvent,
) -> Result<AuthCheckOutcome, Error>
where
    E: Event + Clone + Send + Sync,
    FetchEvent: Fn(OwnedEventId) -> EventFut + Send,
    EventFut: Future<Output = Option<E>> + Send,
{
    let mut cache: FetchCache<OwnedEventId, E> = FetchCache::new();
    let mut pending: Vec<OwnedEventId> =
        incoming_event.auth_events().map(|id| id.borrow().to_owned()).collect();

    // Since room version 12 the create event is identified by the room ID rather
    // than listed in `auth_events`, so it is fetched by the ID derived from it.
    if rules.authorization.room_create_event_id_as_room_id
        && let Some(room_id) = incoming_event.room_id()
        && let Ok(create_event_id) = room_create_event_id(room_id)
    {
        pending.push(create_event_id);
    }

    for round in 0..MAX_FETCH_ROUNDS {
        for event_id in pending.drain(..) {
            if cache.contains(&event_id) {
                continue;
            }

            let event = fetch_event(event_id.clone()).await;
            cache.insert(event_id, event);
        }

        let result = common_ruma::state_res::check_state_independent_auth_rules(
            &rules.authorization,
            incoming_event,
            |event_id: &EventId| cache.get(&event_id.to_owned()),
        );

        match result {
            Ok(()) => return Ok(AuthCheckOutcome::Allow),
            Err(reason) => {
                let misses = cache.take_misses();

                if misses.is_empty() {
                    return Ok(AuthCheckOutcome::Deny(reason));
                }

                debug!(round, misses = misses.len(), "fetching events the auth rules asked for");
                pending.extend(misses);
            }
        }
    }

    Err(Error::FetchRoundsExhausted)
}

/// Checks an event against the state-dependent authorization rules.
///
/// `fetch_state` reads the state snapshot the event is being checked against.
/// The specification runs this check three times for an event — against its
/// `auth_events`, against the state before it, and against the room's current
/// state — so the caller decides which snapshot each call reads.
///
/// The lookups are seeded with the auth types the specification selects for the
/// event, plus the `m.room.create` event, which every state-dependent check
/// reads.
///
/// # Errors
///
/// Returns an error when a lookup the rules depend on could not be resolved
/// within [`MAX_FETCH_ROUNDS`] rounds of fetching.
#[instrument(level = "debug", skip_all, fields(event_id = %incoming_event.event_id().borrow()))]
pub async fn check_state_dependent_auth_rules<E, FetchState, StateFut>(
    rules: &RoomVersionRules,
    incoming_event: &E,
    fetch_state: FetchState,
) -> Result<AuthCheckOutcome, Error>
where
    E: Event + Clone + Send + Sync,
    FetchState: Fn(StateEventType, String) -> StateFut + Send,
    StateFut: Future<Output = Option<E>> + Send,
{
    let auth_types = match auth_types_for_event(
        incoming_event.event_type(),
        incoming_event.sender(),
        incoming_event.state_key(),
        incoming_event.content(),
        &rules.authorization,
    ) {
        Ok(auth_types) => auth_types,
        // The selection algorithm rejects the event's own shape, which is a
        // denial rather than a failure to run the check.
        Err(reason) => return Ok(AuthCheckOutcome::Deny(reason)),
    };

    let mut cache: FetchCache<(StateEventType, String), E> = FetchCache::new();
    let mut pending: Vec<(StateEventType, String)> = auth_types;

    // Every state-dependent check reads the create event, which the selection
    // algorithm omits from room version 12 onwards.
    pending.push((StateEventType::RoomCreate, String::new()));

    for round in 0..MAX_FETCH_ROUNDS {
        for key in pending.drain(..) {
            if cache.contains(&key) {
                continue;
            }

            let event = fetch_state(key.0.clone(), key.1.clone()).await;
            cache.insert(key, event);
        }

        let result = common_ruma::state_res::check_state_dependent_auth_rules(
            &rules.authorization,
            incoming_event,
            |event_type: &StateEventType, state_key: &str| {
                cache.get(&(event_type.clone(), state_key.to_owned()))
            },
        );

        match result {
            Ok(()) => return Ok(AuthCheckOutcome::Allow),
            Err(reason) => {
                let misses = cache.take_misses();

                if misses.is_empty() {
                    return Ok(AuthCheckOutcome::Deny(reason));
                }

                debug!(round, misses = misses.len(), "fetching state the auth rules asked for");
                pending.extend(misses);
            }
        }
    }

    Err(Error::FetchRoundsExhausted)
}

/// The event ID of a room's `m.room.create` event, derived from its room ID.
///
/// Room versions from 12 onwards use the create event's reference hash as the
/// room ID, so the two differ only by their sigil.
fn room_create_event_id(room_id: &RoomId) -> Result<OwnedEventId, common_ruma::IdParseError> {
    EventId::parse(format!("${}", room_id.strip_sigil()))
}
