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

//! A pluggable abstraction over the full-text search engine backing message
//! search.
//!
//! The SDK talks to a [`RoomSearchIndex`] per room, and asks a
//! [`SearchIndexProvider`] for one whenever it meets a room it has not indexed
//! yet. Nothing in these two traits, nor in the types they exchange, is tied
//! to a particular engine.
//!
//! The built-in provider is backed by [Tantivy] and lives behind this crate's
//! `tantivy` feature, which is on by default. Turning it off leaves just this
//! module, so that a client can supply its own engine — SQLite FTS5, the
//! server-side [search endpoint], an external index — without paying for
//! Tantivy, which needs `mmap` and so cannot be built for Wasm.
//!
//! [Tantivy]: https://docs.rs/tantivy
//! [search endpoint]: https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3search

use std::fmt;

use harana_matrix_common::{MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedUserId, RoomId, UInt};

use crate::error::IndexError;

/// The subset of an event's data required to index it and later retrieve it.
///
/// Produced by the matrix layer, which knows how to extract searchable text
/// from each event type. This crate stays agnostic to Matrix event content.
#[derive(Clone)]
pub struct IndexableEvent {
    /// The event's own id (primary key).
    pub(crate) event_id: OwnedEventId,
    /// The id used as the deletion key: the original event id for edits,
    /// otherwise the event's own id.
    pub(crate) original_event_id: OwnedEventId,
    /// The sender of the event.
    pub(crate) sender: OwnedUserId,
    /// The origin server timestamp of the event.
    ///
    /// Please use the `client_common::TimelineEvent::timestamp` as much as
    /// possible as it protects against malformed `origin_server_ts`. At worst,
    /// use the `client_common::serde_helpers::extract_timestamp` function.
    pub(crate) timestamp: Option<MilliSecondsSinceUnixEpoch>,
    /// The text to index for this event.
    pub(crate) body: String,
}

impl fmt::Debug for IndexableEvent {
    /// Don't log bodies
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndexableEvent")
            .field("event_id", &self.event_id)
            .field("original_event_id", &self.original_event_id)
            .field("sender", &self.sender)
            .field("timestamp", &self.timestamp)
            .finish_non_exhaustive()
    }
}

/// Maximum value for the timestamp, so that backends storing it in nanoseconds
/// do not overflow. See [`IndexableEvent::new`] to learn more.
pub(crate) const MAX_MILLISECONDS: u64 = (i64::MAX / 1_000_000).cast_unsigned();

impl IndexableEvent {
    /// Create a new [`IndexableEvent`].
    pub fn new(
        event_id: OwnedEventId,
        original_event_id: OwnedEventId,
        sender: OwnedUserId,
        mut timestamp: Option<MilliSecondsSinceUnixEpoch>,
        body: String,
    ) -> Self {
        // Tantivy will transform the number of milliseconds to nanoseconds
        // by multiplying by 1_000_000 [1]. If the number of milliseconds is too
        // big, the multiplication will overflow.
        //
        // To avoid this panic, we cap the number of milliseconds to a maximum
        // value.
        //
        // [1]: https://github.com/quickwit-oss/tantivy/blob/31ca1a8ba290b425f871d2e2384592045ec01b8d/common/src/datetime.rs#L62-L67
        if let Some(timestamp) = &mut timestamp {
            *timestamp = MilliSecondsSinceUnixEpoch(
                timestamp.get().min(UInt::new_saturating(MAX_MILLISECONDS)),
            );
        }

        Self { event_id, original_event_id, sender, timestamp, body }
    }

    /// The event's own id, the primary key of its document.
    pub fn event_id(&self) -> &OwnedEventId {
        &self.event_id
    }

    /// The id documents are deleted by: the original event id for edits,
    /// otherwise the event's own id.
    pub fn original_event_id(&self) -> &OwnedEventId {
        &self.original_event_id
    }

    /// The sender of the event.
    pub fn sender(&self) -> &OwnedUserId {
        &self.sender
    }

    /// The origin server timestamp of the event, capped so that it can be
    /// stored in nanoseconds without overflowing.
    pub fn timestamp(&self) -> Option<MilliSecondsSinceUnixEpoch> {
        self.timestamp
    }

    /// The text to index for this event.
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// A change to apply to a room's search index.
#[derive(Debug, Clone)]
pub enum RoomIndexOperation {
    /// Add this event to the index.
    Add(IndexableEvent),
    /// Remove every document in the index whose deletion key matches this
    /// event id.
    Remove(OwnedEventId),
    /// Replace every document in the index whose deletion key matches this
    /// event id with the new event.
    Edit(OwnedEventId, IndexableEvent),
    /// Do nothing.
    Noop,
}

/// One room's full-text index.
///
/// Implementations are held per room and are not shared between rooms, but the
/// SDK moves them across tasks, so they have to be `Send` and `Sync`.
pub trait RoomSearchIndex: fmt::Debug + Send + Sync {
    /// Apply one operation to the index, and make the result visible to
    /// subsequent searches.
    ///
    /// Prefer [`RoomSearchIndex::bulk_execute`] for several operations, as
    /// implementations may amortise the cost of committing over a batch.
    fn execute(&mut self, operation: RoomIndexOperation) -> Result<(), IndexError>;

    /// Apply several operations to the index, in order, and make the results
    /// visible to subsequent searches.
    fn bulk_execute(&mut self, operations: Vec<RoomIndexOperation>) -> Result<(), IndexError>;

    /// Search the index, returning at most `max_number_of_results` results as
    /// `(score, event id)` pairs, ordered by descending score.
    ///
    /// `pagination_offset` skips that many of the best-scoring results first,
    /// so an offset of 10 with a limit of 3 returns the 11th, 12th and 13th
    /// results.
    fn search(
        &self,
        query: &str,
        max_number_of_results: usize,
        pagination_offset: Option<usize>,
    ) -> Result<Vec<(f32, OwnedEventId)>, IndexError>;
}

/// Source of the per-room indexes the SDK searches.
///
/// Install one with `ClientBuilder::search_index_provider` to search with an
/// engine of your own.
pub trait SearchIndexProvider: fmt::Debug + Send + Sync {
    /// Create, or reopen, the index for one room.
    ///
    /// Called once per room, the first time the SDK indexes or searches it.
    /// An implementation backed by storage should reopen the room's existing
    /// index rather than starting an empty one.
    fn create_index(&self, room_id: &RoomId) -> Result<Box<dyn RoomSearchIndex>, IndexError>;
}
