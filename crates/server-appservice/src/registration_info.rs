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
// Ported from tuwunel `src/service/appservice/registration_info.rs`.

//! One registration and the namespaces it claims.

use harana_matrix_common::{OwnedUserId, RoomAliasId, RoomId, ServerName, UserId, api::appservice::Registration};

use crate::{Error, NamespaceRegex};

/// An appservice registration combined with its compiled regular expressions.
#[derive(Clone, Debug)]
pub struct RegistrationInfo {
    /// The room alias namespaces this appservice declares.
    pub aliases: NamespaceRegex,

    /// The user namespaces this appservice declares.
    pub users: NamespaceRegex,

    /// The room ID namespaces this appservice declares.
    pub rooms: NamespaceRegex,

    /// The user the appservice acts as, from its `sender_localpart`.
    pub sender: OwnedUserId,

    /// The registration this was built from.
    pub registration: Registration,
}

impl RegistrationInfo {
    /// Compiles a registration's namespaces for the given server.
    ///
    /// Room IDs are matched case-sensitively, since they are opaque; user IDs
    /// and aliases are not, since their localparts are not case-sensitive.
    ///
    /// # Errors
    ///
    /// Returns an error if a namespace pattern does not compile, or if
    /// `sender_localpart` does not form a valid user ID on `server_name`.
    pub fn new(registration: Registration, server_name: &ServerName) -> Result<Self, Error> {
        Ok(Self {
            aliases: NamespaceRegex::new(false, registration.namespaces.aliases.iter())?,
            users: NamespaceRegex::new(false, registration.namespaces.users.iter())?,
            rooms: NamespaceRegex::new(true, registration.namespaces.rooms.iter())?,
            sender: UserId::parse_with_server_name(
                registration.sender_localpart.as_str(),
                server_name,
            )?,

            registration,
        })
    }

    /// Whether the appservice is interested in a user.
    ///
    /// Per [MSC3905] the `users` namespace matches local users only, so a
    /// remote user whose ID happens to match the pattern is not claimed. The
    /// appservice is always interested in its own sender.
    ///
    /// [MSC3905]: https://github.com/matrix-org/matrix-spec-proposals/pull/3905
    #[inline]
    #[must_use]
    pub fn is_user_match(&self, user_id: &UserId) -> bool {
        user_id == self.sender
            || (self.users.is_match(user_id.as_str())
                && user_id.server_name() == self.sender.server_name())
    }

    /// Whether the appservice claims a user exclusively.
    ///
    /// Per [MSC3905] the `users` namespace matches local users only.
    ///
    /// [MSC3905]: https://github.com/matrix-org/matrix-spec-proposals/pull/3905
    #[inline]
    #[must_use]
    pub fn is_exclusive_user_match(&self, user_id: &UserId) -> bool {
        user_id == self.sender
            || (self.users.is_exclusive_match(user_id.as_str())
                && user_id.server_name() == self.sender.server_name())
    }

    /// Whether the appservice is interested in a room alias.
    #[inline]
    #[must_use]
    pub fn is_alias_match(&self, alias: &RoomAliasId) -> bool {
        self.aliases.is_match(alias.as_str())
    }

    /// Whether the appservice claims a room alias exclusively.
    #[inline]
    #[must_use]
    pub fn is_exclusive_alias_match(&self, alias: &RoomAliasId) -> bool {
        self.aliases.is_exclusive_match(alias.as_str())
    }

    /// Whether the appservice is interested in a room.
    #[inline]
    #[must_use]
    pub fn is_room_match(&self, room_id: &RoomId) -> bool {
        self.rooms.is_match(room_id.as_str())
    }

    /// Whether the appservice claims a room exclusively.
    #[inline]
    #[must_use]
    pub fn is_exclusive_room_match(&self, room_id: &RoomId) -> bool {
        self.rooms.is_exclusive_match(room_id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use harana_matrix_common::{
        RoomAliasId, ServerName, UserId,
        api::appservice::{Namespace, Namespaces, Registration, RegistrationInit},
    };

    use super::RegistrationInfo;

    fn registration() -> Registration {
        let mut namespaces = Namespaces::new();
        // Unanchored and without a server part, so it matches a remote ID's text
        // too. That is what makes the MSC3905 scoping observable below.
        namespaces.users = vec![Namespace::new(true, r"@bridge_.*".to_owned())];
        namespaces.aliases = vec![Namespace::new(true, r"#bridge_.*:localhost".to_owned())];

        RegistrationInit {
            id: "bridge".to_owned(),
            url: Some("http://localhost:9000".to_owned()),
            as_token: "as-token".to_owned(),
            hs_token: "hs-token".to_owned(),
            sender_localpart: "bridgebot".to_owned(),
            namespaces,
            rate_limited: None,
            protocols: None,
        }
        .into()
    }

    fn info() -> RegistrationInfo {
        RegistrationInfo::new(registration(), ServerName::parse("localhost").unwrap().as_ref())
            .unwrap()
    }

    #[test]
    fn test_the_sender_is_derived_from_the_localpart() {
        assert_eq!(info().sender, UserId::parse("@bridgebot:localhost").unwrap());
    }

    #[test]
    fn test_the_appservice_claims_its_own_sender() {
        let info = info();
        let sender = UserId::parse("@bridgebot:localhost").unwrap();

        // The sender does not match the namespace pattern, but is claimed all
        // the same.
        assert!(!info.users.is_match(sender.as_str()));
        assert!(info.is_user_match(&sender));
        assert!(info.is_exclusive_user_match(&sender));
    }

    #[test]
    fn test_a_remote_user_is_not_claimed_by_a_matching_pattern() {
        let info = info();
        let local = UserId::parse("@bridge_alice:localhost").unwrap();
        let remote = UserId::parse("@bridge_alice:example.org").unwrap();

        assert!(info.is_user_match(&local));
        assert!(info.is_exclusive_user_match(&local));

        // The pattern matches the remote ID's text, but MSC3905 scopes the
        // namespace to local users.
        assert!(info.users.is_match(remote.as_str()));
        assert!(!info.is_user_match(&remote));
        assert!(!info.is_exclusive_user_match(&remote));
    }

    #[test]
    fn test_aliases_are_matched_against_the_alias_namespace() {
        let info = info();

        assert!(
            info.is_alias_match(RoomAliasId::parse("#bridge_room:localhost").unwrap().as_ref())
        );
        assert!(info.is_exclusive_alias_match(
            RoomAliasId::parse("#bridge_room:localhost").unwrap().as_ref()
        ));
        assert!(!info.is_alias_match(RoomAliasId::parse("#other:localhost").unwrap().as_ref()));
    }
}
