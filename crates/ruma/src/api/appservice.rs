//! (De)serializable types for the [Matrix Application Service
//! API][appservice-api].
//!
//! Only the registration file types are modelled here: they are what a
//! homeserver and an application service agree on out of band, and what
//! [`matrix_sdk_appservice`] needs to decide which users, aliases and rooms a
//! given appservice claims. The push, query and third-party endpoints of the
//! specification are not part of this vendored copy.
//!
//! [appservice-api]: https://spec.matrix.org/v1.19/application-service-api/
//! [`matrix_sdk_appservice`]: https://docs.rs/matrix-sdk-appservice/

use serde::{Deserialize, Serialize};

/// A namespace defined by an application service.
///
/// Used in [`Registration`].
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Namespace {
    /// Whether this namespace is exclusive.
    ///
    /// If true, no other user or application service may claim an ID matching
    /// `regex`, and the homeserver rejects attempts to do so.
    pub exclusive: bool,

    /// A regular expression defining which values this namespace includes.
    pub regex: String,
}

impl Namespace {
    /// Creates a new `Namespace` with the given exclusivity and regex pattern.
    pub fn new(exclusive: bool, regex: String) -> Self {
        Namespace { exclusive, regex }
    }
}

/// Namespaces defined by an application service.
///
/// Used in [`Registration`].
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Namespaces {
    /// Events which are sent from certain users.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<Namespace>,

    /// Events which are sent in rooms with certain room aliases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<Namespace>,

    /// Events which are sent in rooms with certain room IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<Namespace>,
}

impl Namespaces {
    /// Creates a new `Namespaces` instance with empty namespaces.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Information required in the registration yaml file that a homeserver needs.
///
/// To create an instance of this type, first create a [`RegistrationInit`] and
/// convert it via `Registration::from` / `.into()`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Registration {
    /// A unique, user-defined ID of the application service which will never
    /// change.
    pub id: String,

    /// The URL for the application service.
    ///
    /// Optionally set to `None` if no traffic is required.
    pub url: Option<String>,

    /// A unique token for application services to use to authenticate requests
    /// to homeservers.
    pub as_token: String,

    /// A unique token for homeservers to use to authenticate requests to
    /// application services.
    pub hs_token: String,

    /// The localpart of the user associated with the application service.
    pub sender_localpart: String,

    /// A list of users, aliases and rooms namespaces that the application
    /// service controls.
    pub namespaces: Namespaces,

    /// Whether requests from masqueraded users are rate-limited.
    ///
    /// The sender is excluded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limited: Option<bool>,

    /// The external protocols which the application service provides (e.g.
    /// IRC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocols: Option<Vec<String>>,
}

/// Initial set of fields of [`Registration`].
///
/// This struct will not be updated even if additional fields are added to
/// [`Registration`] in a new (non-breaking) release of this crate.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RegistrationInit {
    /// A unique, user-defined ID of the application service which will never
    /// change.
    pub id: String,

    /// The URL for the application service.
    ///
    /// Optionally set to `None` if no traffic is required.
    pub url: Option<String>,

    /// A unique token for application services to use to authenticate requests
    /// to homeservers.
    pub as_token: String,

    /// A unique token for homeservers to use to authenticate requests to
    /// application services.
    pub hs_token: String,

    /// The localpart of the user associated with the application service.
    pub sender_localpart: String,

    /// A list of users, aliases and rooms namespaces that the application
    /// service controls.
    pub namespaces: Namespaces,

    /// Whether requests from masqueraded users are rate-limited.
    ///
    /// The sender is excluded.
    pub rate_limited: Option<bool>,

    /// The external protocols which the application service provides (e.g.
    /// IRC).
    pub protocols: Option<Vec<String>>,
}

impl From<RegistrationInit> for Registration {
    fn from(init: RegistrationInit) -> Self {
        let RegistrationInit {
            id,
            url,
            as_token,
            hs_token,
            sender_localpart,
            namespaces,
            rate_limited,
            protocols,
        } = init;

        Registration {
            id,
            url,
            as_token,
            hs_token,
            sender_localpart,
            namespaces,
            rate_limited,
            protocols,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Registration;

    #[test]
    fn test_registration_deserialization() {
        let registration: Registration = serde_json::from_value(serde_json::json!({
            "id": "IRC Bridge",
            "url": "http://127.0.0.1:1234",
            "as_token": "30c05ae90a248a4188e620216fa72e349803310ec83e2a77b34fe90be6081f46",
            "hs_token": "312df522183efd404ec1cd22d2ffa4bbc76a8c1ccf541dd692eef281356bb74e",
            "sender_localpart": "_irc_bot",
            "namespaces": {
                "users": [{ "exclusive": true, "regex": "@_irc_bridge_.*" }],
                "aliases": [{ "exclusive": false, "regex": "#_irc_bridge_.*" }],
                "rooms": [],
            },
        }))
        .unwrap();

        assert_eq!(registration.id, "IRC Bridge");
        assert_eq!(registration.url.as_deref(), Some("http://127.0.0.1:1234"));
        assert_eq!(registration.sender_localpart, "_irc_bot");
        assert_eq!(registration.namespaces.users.len(), 1);
        assert!(registration.namespaces.users[0].exclusive);
        assert_eq!(registration.namespaces.aliases.len(), 1);
        assert!(!registration.namespaces.aliases[0].exclusive);
        assert!(registration.namespaces.rooms.is_empty());
        // Absent optional fields stay absent rather than failing the parse.
        assert_eq!(registration.rate_limited, None);
        assert_eq!(registration.protocols, None);
    }

    #[test]
    fn test_a_url_less_registration_round_trips() {
        let registration: Registration = serde_json::from_value(serde_json::json!({
            "id": "quiet",
            "url": null,
            "as_token": "as",
            "hs_token": "hs",
            "sender_localpart": "quietbot",
            "namespaces": {},
        }))
        .unwrap();

        assert_eq!(registration.url, None);
        assert!(registration.namespaces.users.is_empty());

        // The empty vectors and the unset options are skipped, so the value we
        // write back is the one a homeserver would accept again.
        let serialized = serde_json::to_value(&registration).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!({
                "id": "quiet",
                "url": null,
                "as_token": "as",
                "hs_token": "hs",
                "sender_localpart": "quietbot",
                "namespaces": {},
            })
        );
    }
}
