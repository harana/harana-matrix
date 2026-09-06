// Copyright 2026 The Matrix.org Foundation C.I.C.
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

//! Detection of replayed Megolm messages.
//!
//! A Megolm ratchet index identifies one message within a session. Nothing in
//! the ciphertext ties it to the event it arrived in, so a server, or anyone
//! else able to inject events into a room, can take a ciphertext it has already
//! seen and hand it back under a new event ID or a new timestamp. Decryption
//! succeeds, and without a check the message shows up a second time, attributed
//! to the original sender, at a place in the timeline the sender never chose.
//!
//! [`ReplayProtection`] remembers which event each `(session ID, ratchet
//! index)` pair was first seen in, and reports a mismatch when the same pair
//! turns up in a different event.

use std::collections::{HashMap, VecDeque};

use harana_matrix_common::{MilliSecondsSinceUnixEpoch, OwnedEventId};

/// How many Megolm sessions we keep replay records for.
///
/// Records are kept in memory only, so this bounds what a room with a lot of
/// key rotation can cost us. Evicting a session's records means a replay of one
/// of its messages is no longer detected, which is the same position we are in
/// after a restart.
const MAX_TRACKED_SESSIONS: usize = 1000;

/// How many ratchet indices we keep replay records for, per session.
const MAX_TRACKED_INDICES_PER_SESSION: usize = 5000;

/// The event a given ratchet index was first decrypted in.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EventFingerprint {
    event_id: OwnedEventId,
    origin_server_ts: MilliSecondsSinceUnixEpoch,
}

/// The replay records of a single Megolm session.
#[derive(Debug, Default)]
struct SessionRecords {
    by_index: HashMap<u32, EventFingerprint>,
    /// Insertion order of `by_index`, so the oldest record can be dropped once
    /// the per-session cap is reached.
    insertion_order: VecDeque<u32>,
}

impl SessionRecords {
    /// Record `fingerprint` at `message_index`, returning the fingerprint we
    /// already had for that index, if any.
    fn record(
        &mut self,
        message_index: u32,
        fingerprint: EventFingerprint,
    ) -> Option<&EventFingerprint> {
        if self.by_index.contains_key(&message_index) {
            // Not `if let`: the borrow of `self.by_index` has to end before the
            // `else` branch can insert into it.
            return self.by_index.get(&message_index);
        }

        if self.insertion_order.len() >= MAX_TRACKED_INDICES_PER_SESSION
            && let Some(oldest) = self.insertion_order.pop_front()
        {
            self.by_index.remove(&oldest);
        }

        self.by_index.insert(message_index, fingerprint);
        self.insertion_order.push_back(message_index);

        None
    }
}

/// The outcome of checking one decrypted event against the replay records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReplayCheck {
    /// This `(session, index)` pair had not been seen, or was last seen in this
    /// very event. Decrypting the same event twice, which happens whenever a
    /// timeline is rebuilt or an event is decrypted again after its key
    /// arrives, is not a replay.
    Ok,

    /// This `(session, index)` pair was first seen in a different event.
    Replayed {
        /// The event the ratchet index was originally decrypted in.
        original_event_id: OwnedEventId,
    },
}

/// Remembers which event each Megolm ratchet index was first seen in.
///
/// The records live in memory for the lifetime of the
/// [`OlmMachine`](crate::OlmMachine). A replayed message that arrives in a
/// later process is not detected, but the copy that is kept and shown is then
/// the first one seen in that process, so the timeline still holds one copy of
/// the message rather than two.
#[derive(Debug, Default)]
pub(crate) struct ReplayProtection {
    sessions: HashMap<String, SessionRecords>,
    /// Insertion order of `sessions`, so the least recently added session can
    /// be dropped once the cap is reached.
    insertion_order: VecDeque<String>,
}

impl ReplayProtection {
    /// Check a freshly decrypted event against the records, and remember it if
    /// this is the first time we see its ratchet index.
    pub(crate) fn check(
        &mut self,
        session_id: &str,
        message_index: u32,
        event_id: &harana_matrix_common::EventId,
        origin_server_ts: MilliSecondsSinceUnixEpoch,
    ) -> ReplayCheck {
        let fingerprint = EventFingerprint { event_id: event_id.to_owned(), origin_server_ts };

        if !self.sessions.contains_key(session_id) {
            if self.insertion_order.len() >= MAX_TRACKED_SESSIONS
                && let Some(oldest) = self.insertion_order.pop_front()
            {
                self.sessions.remove(&oldest);
            }

            self.sessions.insert(session_id.to_owned(), SessionRecords::default());
            self.insertion_order.push_back(session_id.to_owned());
        }

        let records = self.sessions.get_mut(session_id).expect("we just inserted the session");

        match records.record(message_index, fingerprint.clone()) {
            None => ReplayCheck::Ok,
            Some(existing) if *existing == fingerprint => ReplayCheck::Ok,
            Some(existing) => {
                ReplayCheck::Replayed { original_event_id: existing.event_id.clone() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use harana_matrix_common::{MilliSecondsSinceUnixEpoch, event_id};

    use super::{
        MAX_TRACKED_INDICES_PER_SESSION, MAX_TRACKED_SESSIONS, ReplayCheck, ReplayProtection,
    };

    fn ts(millis: u64) -> MilliSecondsSinceUnixEpoch {
        MilliSecondsSinceUnixEpoch(millis.try_into().unwrap())
    }

    #[test]
    fn test_first_sighting_of_an_index_is_accepted() {
        let mut protection = ReplayProtection::default();

        assert_eq!(
            protection.check("session", 0, event_id!("$one:localhost"), ts(1)),
            ReplayCheck::Ok
        );
        assert_eq!(
            protection.check("session", 1, event_id!("$two:localhost"), ts(2)),
            ReplayCheck::Ok
        );
    }

    #[test]
    fn test_decrypting_the_same_event_again_is_not_a_replay() {
        let mut protection = ReplayProtection::default();

        assert_eq!(
            protection.check("session", 0, event_id!("$one:localhost"), ts(1)),
            ReplayCheck::Ok
        );
        assert_eq!(
            protection.check("session", 0, event_id!("$one:localhost"), ts(1)),
            ReplayCheck::Ok,
            "re-decrypting an event we have already seen must not be reported as a replay"
        );
    }

    #[test]
    fn test_same_index_in_a_different_event_is_a_replay() {
        let mut protection = ReplayProtection::default();

        assert_eq!(
            protection.check("session", 0, event_id!("$one:localhost"), ts(1)),
            ReplayCheck::Ok
        );
        assert_eq!(
            protection.check("session", 0, event_id!("$two:localhost"), ts(1)),
            ReplayCheck::Replayed { original_event_id: event_id!("$one:localhost").to_owned() },
        );
    }

    #[test]
    fn test_same_event_id_with_a_moved_timestamp_is_a_replay() {
        let mut protection = ReplayProtection::default();

        assert_eq!(
            protection.check("session", 0, event_id!("$one:localhost"), ts(1)),
            ReplayCheck::Ok
        );
        assert_eq!(
            protection.check("session", 0, event_id!("$one:localhost"), ts(9)),
            ReplayCheck::Replayed { original_event_id: event_id!("$one:localhost").to_owned() },
        );
    }

    #[test]
    fn test_indices_are_tracked_per_session() {
        let mut protection = ReplayProtection::default();

        assert_eq!(
            protection.check("session1", 0, event_id!("$one:localhost"), ts(1)),
            ReplayCheck::Ok
        );
        assert_eq!(
            protection.check("session2", 0, event_id!("$two:localhost"), ts(1)),
            ReplayCheck::Ok,
            "the same index in a different session is a different message"
        );
    }

    #[test]
    fn test_the_number_of_tracked_sessions_is_bounded() {
        let mut protection = ReplayProtection::default();

        for i in 0..MAX_TRACKED_SESSIONS + 1 {
            protection.check(&format!("session{i}"), 0, event_id!("$one:localhost"), ts(1));
        }

        assert_eq!(protection.sessions.len(), MAX_TRACKED_SESSIONS);
        assert!(!protection.sessions.contains_key("session0"), "the oldest session was evicted");
    }

    #[test]
    fn test_the_number_of_tracked_indices_per_session_is_bounded() {
        let mut protection = ReplayProtection::default();

        for i in 0..MAX_TRACKED_INDICES_PER_SESSION as u32 + 1 {
            protection.check("session", i, event_id!("$one:localhost"), ts(1));
        }

        let records = &protection.sessions["session"];
        assert_eq!(records.by_index.len(), MAX_TRACKED_INDICES_PER_SESSION);
        assert!(!records.by_index.contains_key(&0), "the oldest index was evicted");
    }
}
