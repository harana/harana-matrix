//! Types for the [appservice API].
//!
//! Only the registration file format is vendored here: the endpoints of the
//! appservice API are not used by this workspace.
//!
//! [appservice API]: https://spec.matrix.org/latest/application-service-api/

use serde::{Deserialize, Serialize};

/// A namespace defined by an application service.
///
/// Used for [`Namespaces`].
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Namespace {
    /// Whether this application service has exclusive access to matching
    /// events.
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
/// Used for [`Registration`].
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Namespaces {
    /// Events which are sent from certain users.
    #[serde(default)]
    pub users: Vec<Namespace>,

    /// Events which are sent in rooms with certain room aliases.
    #[serde(default)]
    pub aliases: Vec<Namespace>,

    /// Events which are sent in rooms with certain room IDs.
    #[serde(default)]
    pub rooms: Vec<Namespace>,
}

impl Namespaces {
    /// Creates a new `Namespaces` instance with empty namespaces.
    pub fn new() -> Self {
        Self::default()
    }
}

/// A registration is represented by a YAML file provided to each homeserver.
///
/// It defines the namespaces the application service is interested in, and the
/// tokens the homeserver and the application service authenticate each other
/// with.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
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
    pub rate_limited: Option<bool>,

    /// The external protocols which the application service provides (e.g.
    /// IRC).
    pub protocols: Option<Vec<String>>,
}

/// Initial set of fields of [`Registration`].
///
/// This struct will not be updated even if additional fields are added to
/// [`Registration`] in a new (non-breaking) release of the Matrix
/// specification.
#[derive(Clone, Debug)]
#[allow(clippy::exhaustive_structs)]
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
    use serde_json::{json, to_value as to_json_value};

    use super::{Namespace, Namespaces, Registration};

    #[test]
    fn registration_serializes_to_the_documented_shape() {
        let mut namespaces = Namespaces::new();
        namespaces.users = vec![Namespace::new(true, "@_bridge_.*".to_owned())];

        let registration = Registration {
            id: "bridge".to_owned(),
            url: Some("http://localhost:9000".to_owned()),
            as_token: "as-token".to_owned(),
            hs_token: "hs-token".to_owned(),
            sender_localpart: "bridgebot".to_owned(),
            namespaces,
            rate_limited: None,
            protocols: None,
        };

        assert_eq!(
            to_json_value(&registration).unwrap(),
            json!({
                "id": "bridge",
                "url": "http://localhost:9000",
                "as_token": "as-token",
                "hs_token": "hs-token",
                "sender_localpart": "bridgebot",
                "namespaces": {
                    "users": [{ "exclusive": true, "regex": "@_bridge_.*" }],
                    "aliases": [],
                    "rooms": [],
                },
                "rate_limited": null,
                "protocols": null,
            })
        );
    }

    #[test]
    fn a_registration_without_namespaces_deserializes() {
        let registration: Registration = serde_json::from_value(json!({
            "id": "bridge",
            "url": null,
            "as_token": "as-token",
            "hs_token": "hs-token",
            "sender_localpart": "bridgebot",
            "namespaces": {},
        }))
        .unwrap();

        assert!(registration.namespaces.users.is_empty());
        assert!(registration.namespaces.aliases.is_empty());
        assert!(registration.namespaces.rooms.is_empty());
        assert_eq!(registration.url, None);
    }
}
