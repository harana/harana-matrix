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

//! Detection of replayed Megolm events.
//!
//! Every message encrypted with a Megolm session carries a message index, and
//! a given index is only ever used for one message. An attacker who can inject
//! events into a room can therefore take a ciphertext the room has already
//! seen and present it again as a new event: it decrypts perfectly, and
//! without a check like this one it is shown as a new message, attributed to
//! the original sender, at a time of the attacker's choosing.

use std::collections::{HashMap, VecDeque};

use matrix_sdk_common::locks::Mutex as StdMutex;
use ruma::{EventId, OwnedEventId};
use tracing::warn;

/// How many `(session, message index)` pairs we remember.
///
/// This is a bounded, in-memory record, so a long-lived client eventually
/// forgets the oldest events it has seen. The entries are small and this is
/// generous enough to cover the events a user is realistically looking at.
const MAX_TRACKED_MESSAGE_INDICES: usize = 20_000;

/// A Megolm message index we have already decrypted, and the event it belonged
/// to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MessageIndexKey {
    session_id: String,
    message_index: u32,
}

/// Remembers which Megolm message indices we have already seen, so that the
/// same index turning up on a different event can be spotted.
#[derive(Debug, Default)]
pub(crate) struct ReplayProtection {
    inner: StdMutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    seen: HashMap<MessageIndexKey, OwnedEventId>,
    /// The keys in insertion order, so that the oldest can be dropped once we
    /// reach [`MAX_TRACKED_MESSAGE_INDICES`].
    insertion_order: VecDeque<MessageIndexKey>,
}

impl ReplayProtection {
    /// Record that the given event was decrypted with the given Megolm session
    /// and message index.
    ///
    /// Returns the event ID we had already recorded for this
    /// `(session, message index)` pair if it is a *different* event, i.e. if
    /// this looks like a replay. Decrypting the same event again, which
    /// happens routinely, is not a replay and returns `None`.
    pub(crate) fn check_and_record(
        &self,
        session_id: &str,
        message_index: u32,
        event_id: &EventId,
    ) -> Option<OwnedEventId> {
        let key = MessageIndexKey { session_id: session_id.to_owned(), message_index };

        let mut inner = self.inner.lock();

        if let Some(known_event_id) = inner.seen.get(&key) {
            return (known_event_id != event_id).then(|| known_event_id.to_owned());
        }

        if inner.insertion_order.len() >= MAX_TRACKED_MESSAGE_INDICES
            && let Some(oldest) = inner.insertion_order.pop_front()
        {
            inner.seen.remove(&oldest);
        }

        inner.seen.insert(key.clone(), event_id.to_owned());
        inner.insertion_order.push_back(key);

        None
    }

    /// Log that we have spotted a replayed event.
    pub(crate) fn warn_about_replay(
        session_id: &str,
        message_index: u32,
        event_id: &EventId,
        original_event_id: &EventId,
    ) {
        warn!(
            session_id,
            message_index,
            ?event_id,
            ?original_event_id,
            "Refusing to decrypt an event which reuses the Megolm message index of \
             another event: it is a replay of that event"
        );
    }
}

#[cfg(test)]
mod tests {
    use ruma::event_id;

    use super::{MAX_TRACKED_MESSAGE_INDICES, ReplayProtection};

    #[test]
    fn test_first_use_of_an_index_is_accepted() {
        let protection = ReplayProtection::default();

        assert_eq!(protection.check_and_record("session", 0, event_id!("$1")), None);
        assert_eq!(protection.check_and_record("session", 1, event_id!("$2")), None);
        // A different session may use the same index.
        assert_eq!(protection.check_and_record("other", 0, event_id!("$3")), None);
    }

    #[test]
    fn test_decrypting_the_same_event_again_is_not_a_replay() {
        let protection = ReplayProtection::default();

        assert_eq!(protection.check_and_record("session", 0, event_id!("$1")), None);
        assert_eq!(protection.check_and_record("session", 0, event_id!("$1")), None);
    }

    #[test]
    fn test_reusing_an_index_for_another_event_is_a_replay() {
        let protection = ReplayProtection::default();

        assert_eq!(protection.check_and_record("session", 0, event_id!("$1")), None);
        assert_eq!(
            protection.check_and_record("session", 0, event_id!("$2")),
            Some(event_id!("$1").to_owned()),
            "The index was already used by another event"
        );
    }

    #[test]
    fn test_the_record_is_bounded() {
        let protection = ReplayProtection::default();

        for index in 0..MAX_TRACKED_MESSAGE_INDICES + 10 {
            let event_id = format!("$event{index}");
            let event_id = <&ruma::EventId>::try_from(event_id.as_str()).unwrap();

            assert_eq!(protection.check_and_record("session", index as u32, event_id), None);
        }

        let inner = protection.inner.lock();
        assert_eq!(inner.seen.len(), MAX_TRACKED_MESSAGE_INDICES);
        assert_eq!(inner.insertion_order.len(), MAX_TRACKED_MESSAGE_INDICES);
    }
}
