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

use std::{borrow::Borrow, future::Future};

use harana_matrix_common::{
    EventId, OwnedEventId,
    room_version_rules::RoomVersionRules,
    state_res::{Event, StateMap, utils::event_id_set::EventIdSet},
};
use tracing::{debug, instrument};

use crate::{
    Error,
    fetch::{FetchCache, MAX_FETCH_ROUNDS},
};

/// Resolves conflicting state maps into one room state.
///
/// This is [`harana_matrix_common::state_res::resolve`] driven against an asynchronous store.
/// The lookups are seeded with every event named by `state_maps` and
/// `auth_chains`, which is what the algorithm reads in the ordinary case; if it
/// asks for anything else, that is fetched and resolution is re-run, up to
/// [`MAX_FETCH_ROUNDS`] times.
///
/// `fetch_conflicted_state_subgraph` is only consulted by room versions whose
/// state resolution rules use it, and returning `None` from it fails resolution
/// for those versions.
///
/// # Invariants
///
/// Every event must belong to the same room.
///
/// # Errors
///
/// Returns an error when resolution fails, or when an event it depends on could
/// not be resolved within [`MAX_FETCH_ROUNDS`] rounds of fetching.
#[instrument(level = "debug", skip_all, fields(state_maps = state_maps.len()))]
pub async fn resolve<E, FetchEvent, EventFut, FetchSubgraph>(
    rules: &RoomVersionRules,
    state_maps: &[StateMap<E::Id>],
    auth_chains: Vec<EventIdSet<E::Id>>,
    fetch_event: FetchEvent,
    fetch_conflicted_state_subgraph: FetchSubgraph,
) -> Result<StateMap<E::Id>, Error>
where
    E: Event + Clone + Send + Sync,
    E::Id: Clone + Send + Sync,
    FetchEvent: Fn(OwnedEventId) -> EventFut + Send,
    EventFut: Future<Output = Option<E>> + Send,
    FetchSubgraph: Fn(&StateMap<Vec<E::Id>>) -> Option<EventIdSet<E::Id>> + Send,
{
    // Ruma implements the second version of the algorithm, which every room
    // version but the first uses.
    let state_res_rules =
        rules.state_res.v2_rules().ok_or(Error::UnsupportedStateResolutionVersion)?;

    let mut cache: FetchCache<OwnedEventId, E> = FetchCache::new();

    // Everything the state maps and auth chains name is read in the ordinary
    // case, so it is all fetched up front rather than a round at a time.
    let mut pending: Vec<OwnedEventId> = state_maps
        .iter()
        .flat_map(|state_map| state_map.values())
        .chain(auth_chains.iter().flat_map(EventIdSet::iter))
        .map(|event_id| event_id.borrow().to_owned())
        .collect();

    for round in 0..MAX_FETCH_ROUNDS {
        for event_id in pending.drain(..) {
            if cache.contains(&event_id) {
                continue;
            }

            let event = fetch_event(event_id.clone()).await;
            cache.insert(event_id, event);
        }

        let result = harana_matrix_common::state_res::resolve(
            &rules.authorization,
            state_res_rules,
            state_maps.iter(),
            auth_chains.clone(),
            |event_id: &EventId| cache.get(&event_id.to_owned()),
            &fetch_conflicted_state_subgraph,
        );

        let misses = cache.take_misses();

        match result {
            Ok(state) => return Ok(state),
            Err(error) => {
                if misses.is_empty() {
                    return Err(error.into());
                }

                debug!(round, misses = misses.len(), "fetching events state resolution asked for");
                pending.extend(misses);
            }
        }
    }

    Err(Error::FetchRoundsExhausted)
}
