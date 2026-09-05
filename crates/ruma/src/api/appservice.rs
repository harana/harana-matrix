//! Types for the [Application Service API].
//!
//! Only the registration-file types are vendored here. The appservice HTTP
//! endpoints themselves (`/transactions`, `/users`, `/rooms`, `/ping` and the
//! third-party lookups) are not used by this workspace and are not included.
//!
//! [Application Service API]: https://spec.matrix.org/v1.18/application-service-api/

use serde::{Deserialize, Serialize};

/// A namespace defined by an application service.
///
/// Used for [appservice registration](https://spec.matrix.org/v1.18/application-service-api/#registration).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Namespace {
    /// Whether this application service has exclusive access to events within this namespace.
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
/// Used for [appservice registration](https://spec.matrix.org/v1.18/application-service-api/#registration).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    /// Creates a new `Namespaces` instance with empty namespaces for `users`,  `aliases` and
    /// `rooms` (none of them are explicitly required)
    pub fn new() -> Self {
        Self::default()
    }
}

/// Information required in the registration yaml file that a homeserver needs.
///
/// To create an instance of this type, first create a `RegistrationInit` and convert it via
/// `Registration::from` / `.into()`.
///
/// Used for [appservice registration](https://spec.matrix.org/v1.18/application-service-api/#registration).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Registration {
    /// A unique, user - defined ID of the application service which will never change.
    pub id: String,

    /// The URL for the application service.
    ///
    /// Optionally set to `null` if no traffic is required.
    #[serde(deserialize_with = "Option::deserialize")]
    pub url: Option<String>,

    /// A unique token for application services to use to authenticate requests to Homeservers.
    pub as_token: String,

    /// A unique token for Homeservers to use to authenticate requests to application services.
    pub hs_token: String,

    /// The localpart of the user associated with the application service.
    pub sender_localpart: String,

    /// A list of users, aliases and rooms namespaces that the application service controls.
    pub namespaces: Namespaces,

    /// Whether requests from masqueraded users are rate-limited.
    ///
    /// The sender is excluded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limited: Option<bool>,

    /// The external protocols which the application service provides (e.g. IRC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocols: Option<Vec<String>>,

    /// Whether the application service wants to receive ephemeral data.
    ///
    /// Defaults to `false`.
    #[serde(default, skip_serializing_if = "crate::serde::is_default")]
    pub receive_ephemeral: bool,
}

/// Initial set of fields of `Registration`.
///
/// This struct will not be updated even if additional fields are added to `Registration` in a new
/// (non-breaking) release of the Matrix specification.
///
/// Used for [appservice registration](https://spec.matrix.org/v1.18/application-service-api/#registration).
#[derive(Debug)]
#[allow(clippy::exhaustive_structs)]
pub struct RegistrationInit {
    /// A unique, user - defined ID of the application service which will never change.
    pub id: String,

    /// The URL for the application service.
    ///
    /// Optionally set to `null` if no traffic is required.
    pub url: Option<String>,

    /// A unique token for application services to use to authenticate requests to Homeservers.
    pub as_token: String,

    /// A unique token for Homeservers to use to authenticate requests to application services.
    pub hs_token: String,

    /// The localpart of the user associated with the application service.
    pub sender_localpart: String,

    /// A list of users, aliases and rooms namespaces that the application service controls.
    pub namespaces: Namespaces,

    /// Whether requests from masqueraded users are rate-limited.
    ///
    /// The sender is excluded.
    pub rate_limited: Option<bool>,

    /// The external protocols which the application service provides (e.g. IRC).
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
        Self {
            id,
            url,
            as_token,
            hs_token,
            sender_localpart,
            namespaces,
            rate_limited,
            protocols,
            receive_ephemeral: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Namespace, Namespaces, Registration, RegistrationInit};

    fn registration() -> Registration {
        let mut namespaces = Namespaces::new();
        namespaces.users = vec![Namespace::new(true, r"@bridge_.*:localhost".to_owned())];

        RegistrationInit {
            id: "bridge".to_owned(),
            url: Some("http://localhost:9000".to_owned()),
            as_token: "as-token".to_owned(),
            hs_token: "hs-token".to_owned(),
            sender_localpart: "bridgebot".to_owned(),
            namespaces,
            rate_limited: Some(false),
            protocols: None,
        }
        .into()
    }

    #[test]
    fn test_registration_init_defaults_receive_ephemeral_to_false() {
        assert!(!registration().receive_ephemeral);
    }

    #[test]
    fn test_registration_round_trips_through_json() {
        let registration = registration();
        let json = serde_json::to_value(&registration).unwrap();
        let parsed: Registration = serde_json::from_value(json).unwrap();

        assert_eq!(parsed.id, registration.id);
        assert_eq!(parsed.sender_localpart, registration.sender_localpart);
        assert_eq!(parsed.namespaces.users.len(), 1);
        assert!(parsed.namespaces.users[0].exclusive);
        assert!(parsed.namespaces.aliases.is_empty());
    }

    #[test]
    fn test_absent_namespaces_deserialize_as_empty() {
        let parsed: Registration = serde_json::from_str(
            r#"{
                "id": "bridge",
                "url": null,
                "as_token": "as",
                "hs_token": "hs",
                "sender_localpart": "bridgebot",
                "namespaces": {}
            }"#,
        )
        .unwrap();

        assert!(parsed.url.is_none());
        assert!(parsed.namespaces.users.is_empty());
        assert!(parsed.namespaces.aliases.is_empty());
        assert!(parsed.namespaces.rooms.is_empty());
    }
}
