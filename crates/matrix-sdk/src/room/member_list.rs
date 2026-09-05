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

//! Querying the member list of a room: filtering, sorting and pagination.
//!
//! See [`Room::member_list`].
//!
//! [`Room::member_list`]: crate::Room::member_list

use matrix_sdk_base::{RoomMemberships, RoomMembersUpdate};
use ruma::events::room::power_levels::UserPowerLevel;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};

use super::{Room, RoomMember};
use crate::Result;

/// How to order the members of a room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum RoomMemberSortOrder {
    /// By display name, case-insensitively, and by user ID between members
    /// that share one.
    #[default]
    Name,

    /// By power level, highest first, and by name between members that share
    /// one.
    ///
    /// This is what a member list that shows administrators and moderators
    /// above everyone else needs.
    PowerLevelThenName,
}

/// One page of the member list of a room. See [`RoomMemberListQuery::page`].
#[derive(Debug)]
pub struct RoomMemberListPage {
    /// The members in this page.
    pub members: Vec<RoomMember>,

    /// How many members match the query in total, ignoring the bounds of this
    /// page.
    ///
    /// This is what a paginated view needs to know how far it can scroll.
    pub total: usize,
}

/// A query over the members of a room: which ones, in what order, and which
/// page of them.
///
/// Build one with [`Room::member_list`], then read it with
/// [`RoomMemberListQuery::page`] or [`RoomMemberListQuery::all`].
///
/// The members are read from the state store and filtered and sorted in
/// memory, so paginating avoids handing a whole member list to the caller —
/// which is what makes it worth doing across an FFI boundary — but does not
/// avoid loading the room's members from the store.
#[derive(Debug)]
pub struct RoomMemberListQuery {
    room: Room,
    memberships: RoomMemberships,
    search: Option<String>,
    min_power_level: Option<i64>,
    sort: RoomMemberSortOrder,
    sync_members: bool,
}

impl RoomMemberListQuery {
    pub(crate) fn new(room: Room) -> Self {
        Self {
            room,
            memberships: RoomMemberships::empty(),
            search: None,
            min_power_level: None,
            sort: RoomMemberSortOrder::default(),
            sync_members: true,
        }
    }

    /// Only keep the members with one of these memberships.
    ///
    /// [`RoomMemberships::empty()`], the default, keeps all of them.
    pub fn memberships(mut self, memberships: RoomMemberships) -> Self {
        self.memberships = memberships;
        self
    }

    /// Only keep the members whose display name or user ID contains this term,
    /// case-insensitively.
    ///
    /// An empty term keeps all of them.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        let term = term.into();
        self.search = (!term.trim().is_empty()).then(|| term.trim().to_lowercase());
        self
    }

    /// Only keep the members whose power level is at least this high.
    ///
    /// This is what showing the administrators of a room separately needs. A
    /// member with an infinite power level, i.e. a room creator from room
    /// version 12 onwards, always passes.
    pub fn min_power_level(mut self, level: i64) -> Self {
        self.min_power_level = Some(level);
        self
    }

    /// Order the members this way. Defaults to [`RoomMemberSortOrder::Name`].
    pub fn sort_by(mut self, sort: RoomMemberSortOrder) -> Self {
        self.sort = sort;
        self
    }

    /// Answer from the state store alone, without fetching the member list
    /// from the homeserver first.
    ///
    /// Members may then be missing, because of lazy loading.
    pub fn no_sync(mut self) -> Self {
        self.sync_members = false;
        self
    }

    /// Run the query and return every matching member.
    pub async fn all(self) -> Result<Vec<RoomMember>> {
        let sort = self.sort;
        let mut members = self.matching_members().await?;
        sort_members(&mut members, sort);

        Ok(members)
    }

    /// Run the query and return how many members match it.
    pub async fn count(self) -> Result<usize> {
        Ok(self.matching_members().await?.len())
    }

    /// Run the query and return one page of matching members, along with the
    /// total number of matches.
    ///
    /// `offset` members are skipped, then at most `limit` are returned. An
    /// offset past the end yields an empty page rather than an error, so a
    /// caller that scrolls past the end of a shrinking list is not punished
    /// for it.
    pub async fn page(self, offset: usize, limit: usize) -> Result<RoomMemberListPage> {
        let sort = self.sort;
        let mut members = self.matching_members().await?;
        let total = members.len();

        sort_members(&mut members, sort);

        let members = members.into_iter().skip(offset).take(limit).collect();

        Ok(RoomMemberListPage { members, total })
    }

    async fn matching_members(self) -> Result<Vec<RoomMember>> {
        let members = if self.sync_members {
            self.room.members(self.memberships).await?
        } else {
            self.room.members_no_sync(self.memberships).await?
        };

        Ok(members
            .into_iter()
            .filter(|member| {
                self.search.as_ref().is_none_or(|term| matches_search(member, term))
                    && self
                        .min_power_level
                        .is_none_or(|level| power_level_at_least(member, level))
            })
            .collect())
    }
}

impl Room {
    /// Query the members of this room.
    ///
    /// The returned query filters, sorts and paginates the member list; see
    /// [`RoomMemberListQuery`]. To be told when the member list changes, see
    /// [`Room::subscribe_to_member_updates`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use matrix_sdk::{RoomMemberships, room::RoomMemberSortOrder};
    /// # async {
    /// # let room: matrix_sdk::Room = unimplemented!();
    /// // The first 50 joined members, moderators and administrators first.
    /// let page = room
    ///     .member_list()
    ///     .memberships(RoomMemberships::JOIN)
    ///     .sort_by(RoomMemberSortOrder::PowerLevelThenName)
    ///     .page(0, 50)
    ///     .await?;
    ///
    /// println!("showing {} of {} members", page.members.len(), page.total);
    /// # anyhow::Ok(()) };
    /// ```
    pub fn member_list(&self) -> RoomMemberListQuery {
        RoomMemberListQuery::new(self.clone())
    }

    /// Subscribe to the member list of this room.
    ///
    /// The stream yields an item every time members join, leave or change
    /// their profile, so a member list built with [`Room::member_list`] knows
    /// when to run its query again.
    ///
    /// The stream is lagging-tolerant: when updates are produced faster than
    /// they are consumed, the ones that could not be delivered are reported as
    /// a [`RoomMembersUpdate::FullReload`], since a consumer that missed
    /// updates has to reload anyway.
    pub fn subscribe_to_member_updates(
        &self,
    ) -> impl futures_core::Stream<Item = RoomMembersUpdate> + use<> {
        use futures_util::StreamExt as _;

        BroadcastStream::new(self.inner.room_member_updates_sender.subscribe()).map(|update| {
            match update {
                Ok(update) => update,
                // Whatever was missed, the only safe answer is to reload.
                Err(BroadcastStreamRecvError::Lagged(_)) => RoomMembersUpdate::FullReload,
            }
        })
    }
}

fn matches_search(member: &RoomMember, term: &str) -> bool {
    member.display_name().is_some_and(|name| name.to_lowercase().contains(term))
        || member.user_id().as_str().to_lowercase().contains(term)
}

fn power_level_at_least(member: &RoomMember, level: i64) -> bool {
    match member.power_level() {
        UserPowerLevel::Infinite => true,
        UserPowerLevel::Int(power_level) => i64::from(power_level) >= level,
        _ => false,
    }
}

fn sort_members(members: &mut [RoomMember], sort: RoomMemberSortOrder) {
    match sort {
        RoomMemberSortOrder::Name => {
            members.sort_by(|a, b| compare_by_name(a, b));
        }
        RoomMemberSortOrder::PowerLevelThenName => {
            members.sort_by(|a, b| {
                compare_power_levels(b.power_level(), a.power_level())
                    .then_with(|| compare_by_name(a, b))
            });
        }
    }
}

fn compare_by_name(a: &RoomMember, b: &RoomMember) -> std::cmp::Ordering {
    a.name()
        .to_lowercase()
        .cmp(&b.name().to_lowercase())
        .then_with(|| a.user_id().cmp(b.user_id()))
}

fn compare_power_levels(a: UserPowerLevel, b: UserPowerLevel) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (UserPowerLevel::Infinite, UserPowerLevel::Infinite) => Ordering::Equal,
        (UserPowerLevel::Infinite, _) => Ordering::Greater,
        (_, UserPowerLevel::Infinite) => Ordering::Less,
        (UserPowerLevel::Int(a), UserPowerLevel::Int(b)) => a.cmp(&b),
        // `UserPowerLevel` is non-exhaustive; an unknown variant sorts with the
        // finite ones rather than above them.
        _ => Ordering::Equal,
    }
}
