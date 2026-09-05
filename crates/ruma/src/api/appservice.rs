//! (De)serializable types for the [Matrix Application Service API][appservice-api].
//!
//! Only the registration file is vendored here: the transaction endpoints the
//! homeserver pushes to an appservice are not used by this workspace.
//!
//! [appservice-api]: https://spec.matrix.org/v1.19/application-service-api/

use serde::{Deserialize, Serialize};

/// A namespace defined by an application service.
///
/// Used in [`Namespaces`].
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct Namespace {
    /// Whether this application service has exclusive access to events within
    /// this namespace.
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

/// A registration file, as it is read by the homeserver and by the application
/// service itself.
///
/// Create this with [`RegistrationInit`].
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
    pub rate_limited: Option<bool>,

    /// The external protocols which the application service provides (e.g.
    /// IRC).
    pub protocols: Option<Vec<String>>,

    /// Whether the application service wants to receive ephemeral data.
    ///
    /// Defaults to `false`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub receive_ephemeral: bool,
}

/// Initial set of fields of [`Registration`].
///
/// This struct will not be updated even if additional fields are added to
/// `Registration` in a new (non-breaking) release of the Matrix specification.
#[derive(Clone, Debug)]
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
            receive_ephemeral: false,
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
