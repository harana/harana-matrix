// Copyright 2025 Tuwunel Contributors
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
//
// Ported from the lookups in tuwunel `src/service/appservice/mod.rs`, without
// its storage or locking: this type is the loaded set, and the caller decides
// how it is shared.

//! The loaded set of registrations, and the questions asked across all of them.

use std::collections::{BTreeMap, btree_map::Values};

use ruma::{RoomAliasId, RoomId, ServerName, UserId, api::appservice::Registration};

use crate::{Error, RegistrationInfo};

/// The appservice registrations a server has loaded, keyed by registration ID.
#[derive(Clone, Debug, Default)]
pub struct Registrations {
    registrations: BTreeMap<String, RegistrationInfo>,
}

impl Registrations {
    /// Creates an empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compiles a registration and adds it, replacing one with the same ID.
    ///
    /// Returns the registration it replaced, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the registration's namespaces do not compile, or if
    /// its `sender_localpart` does not form a valid user ID on `server_name`.
    pub fn insert(
        &mut self,
        registration: Registration,
        server_name: &ServerName,
    ) -> Result<Option<RegistrationInfo>, Error> {
        let info = RegistrationInfo::new(registration, server_name)?;

        Ok(self.registrations.insert(info.registration.id.clone(), info))
    }

    /// Removes a registration by ID, returning it if it was loaded.
    pub fn remove(&mut self, id: &str) -> Option<RegistrationInfo> {
        self.registrations.remove(id)
    }

    /// Returns a registration by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&RegistrationInfo> {
        self.registrations.get(id)
    }

    /// Returns every loaded registration.
    pub fn iter(&self) -> Values<'_, String, RegistrationInfo> {
        self.registrations.values()
    }

    /// The number of loaded registrations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    /// Whether no registration is loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }

    /// Finds the registration an appservice authenticated with.
    ///
    /// The `as_token` is what an appservice presents when it calls the
    /// homeserver, so this is the lookup that authenticates such a request.
    #[must_use]
    pub fn find_from_as_token(&self, token: &str) -> Option<&RegistrationInfo> {
        self.iter().find(|info| info.registration.as_token == token)
    }

    /// Finds the registration a homeserver request should be authenticated
    /// with.
    ///
    /// The `hs_token` is what the homeserver presents when it pushes
    /// transactions to an appservice, so this is the lookup an appservice uses
    /// to authenticate an incoming transaction.
    #[must_use]
    pub fn find_from_hs_token(&self, token: &str) -> Option<&RegistrationInfo> {
        self.iter().find(|info| info.registration.hs_token == token)
    }

    /// Whether any appservice is interested in a user.
    #[must_use]
    pub fn is_interested_in_user(&self, user_id: &UserId) -> bool {
        self.iter().any(|info| info.is_user_match(user_id))
    }

    /// Whether any appservice claims a user exclusively.
    ///
    /// A local user ID in an exclusive namespace cannot be registered by
    /// anyone else, so this is the check a homeserver runs at registration.
    #[must_use]
    pub fn is_exclusive_user_id(&self, user_id: &UserId) -> bool {
        self.iter().any(|info| info.is_exclusive_user_match(user_id))
    }

    /// Whether any appservice claims a room alias exclusively.
    #[must_use]
    pub fn is_exclusive_alias(&self, alias: &RoomAliasId) -> bool {
        self.iter().any(|info| info.is_exclusive_alias_match(alias))
    }

    /// Whether any appservice claims a room exclusively.
    #[must_use]
    pub fn is_exclusive_room_id(&self, room_id: &RoomId) -> bool {
        self.iter().any(|info| info.is_exclusive_room_match(room_id))
    }
}

#[cfg(test)]
mod tests {
    use ruma::{
        ServerName, UserId,
        api::appservice::{Namespace, Namespaces, Registration, RegistrationInit},
    };

    use super::Registrations;

    fn registration(id: &str, prefix: &str, exclusive: bool) -> Registration {
        let mut namespaces = Namespaces::new();
        namespaces.users = vec![Namespace::new(exclusive, format!(r"@{prefix}_.*:localhost"))];

        RegistrationInit {
            id: id.to_owned(),
            url: None,
            as_token: format!("{id}-as-token"),
            hs_token: format!("{id}-hs-token"),
            sender_localpart: format!("{id}bot"),
            namespaces,
            rate_limited: None,
            protocols: None,
        }
        .into()
    }

    fn registrations() -> Registrations {
        let server_name = ServerName::parse("localhost").unwrap();
        let mut registrations = Registrations::new();

        registrations.insert(registration("irc", "irc", true), &server_name).unwrap();
        registrations.insert(registration("watch", "watch", false), &server_name).unwrap();

        registrations
    }

    #[test]
    fn test_a_replaced_registration_is_returned() {
        let server_name = ServerName::parse("localhost").unwrap();
        let mut registrations = registrations();

        let replaced = registrations
            .insert(registration("irc", "ircnet", true), &server_name)
            .unwrap()
            .expect("the first irc registration is replaced");

        assert_eq!(replaced.registration.sender_localpart, "ircbot");
        assert_eq!(registrations.len(), 2);
    }

    #[test]
    fn test_tokens_identify_their_registration() {
        let registrations = registrations();

        assert_eq!(
            registrations.find_from_as_token("irc-as-token").unwrap().registration.id,
            "irc"
        );
        assert_eq!(
            registrations.find_from_hs_token("watch-hs-token").unwrap().registration.id,
            "watch"
        );
        assert!(registrations.find_from_as_token("irc-hs-token").is_none());
        assert!(registrations.find_from_as_token("unknown").is_none());
    }

    #[test]
    fn test_interest_is_wider_than_an_exclusive_claim() {
        let registrations = registrations();
        let watched = UserId::parse("@watch_alice:localhost").unwrap();
        let claimed = UserId::parse("@irc_alice:localhost").unwrap();

        assert!(registrations.is_interested_in_user(&watched));
        assert!(!registrations.is_exclusive_user_id(&watched));

        assert!(registrations.is_interested_in_user(&claimed));
        assert!(registrations.is_exclusive_user_id(&claimed));
    }

    #[test]
    fn test_a_removed_registration_no_longer_claims_anything() {
        let mut registrations = registrations();
        let claimed = UserId::parse("@irc_alice:localhost").unwrap();

        assert!(registrations.remove("irc").is_some());
        assert!(!registrations.is_exclusive_user_id(&claimed));
        assert!(registrations.remove("irc").is_none());
    }
}
