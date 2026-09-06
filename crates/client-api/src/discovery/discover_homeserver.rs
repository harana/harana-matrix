//! `GET /.well-known/matrix/client` ([spec])
//!
//! [spec]: https://spec.matrix.org/v1.19/client-server-api/#getwell-knownmatrixclient
//!
//! Get discovery information about the domain.

use serde::{Deserialize, Serialize};

#[cfg(feature = "unstable-msc4143")]
use crate::rtc::RtcTransport;
use crate::__ruma::{
    api::{auth_scheme::NoAccessToken, request, response},
    metadata,
};

metadata! {
    method: GET,
    rate_limited: false,
    authentication: NoAccessToken,
    path: "/.well-known/matrix/client",
}

/// Request type for the `client_well_known` endpoint.
#[request]
#[derive(Default)]
pub struct Request {}

/// Response type for the `client_well_known` endpoint.
///
/// The body is (de)serialized by hand, see [`ResponseBody`].
#[response(manual_body_serde)]
pub struct Response {
    /// Information about the homeserver to connect to.
    pub homeserver: HomeserverInfo,

    /// Information about the identity server to connect to.
    pub identity_server: Option<IdentityServerInfo>,

    /// Information about the tile server to use to display location data.
    #[cfg(feature = "unstable-msc3488")]
    pub tile_server: Option<TileServerInfo>,

    /// A list of the available MatrixRTC foci, ordered by priority.
    #[cfg(feature = "unstable-msc4143")]
    pub rtc_foci: Vec<RtcTransport>,
}

/// The `.well-known` document as it is read off the wire.
///
/// A well-known file is written by the server's administrator, not generated,
/// and while an MSC is being stabilised it is common to see the same value
/// listed under both its unstable and its stable name. `#[serde(alias)]` maps
/// both names onto one field and then rejects the second one as a duplicate,
/// which fails the whole document and leaves the client with no discovery
/// information at all. So each name is read as a field of its own here and the
/// pair is reconciled afterwards, preferring the stable name.
#[derive(Deserialize)]
struct ResponseBodyDeHelper {
    #[serde(rename = "m.homeserver")]
    homeserver: HomeserverInfo,

    #[serde(rename = "m.identity_server", default)]
    identity_server: Option<IdentityServerInfo>,

    #[cfg(feature = "unstable-msc3488")]
    #[serde(rename = "m.tile_server", default)]
    stable_tile_server: Option<TileServerInfo>,

    #[cfg(feature = "unstable-msc3488")]
    #[serde(rename = "org.matrix.msc3488.tile_server", default)]
    unstable_tile_server: Option<TileServerInfo>,

    #[cfg(feature = "unstable-msc4143")]
    #[serde(rename = "m.rtc_foci", default)]
    stable_rtc_foci: Option<Vec<RtcTransport>>,

    #[cfg(feature = "unstable-msc4143")]
    #[serde(rename = "org.matrix.msc4143.rtc_foci", default)]
    unstable_rtc_foci: Option<Vec<RtcTransport>>,
}

impl<'de> Deserialize<'de> for ResponseBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = ResponseBodyDeHelper::deserialize(deserializer)?;

        Ok(Self {
            homeserver: helper.homeserver,
            identity_server: helper.identity_server,
            #[cfg(feature = "unstable-msc3488")]
            tile_server: helper.stable_tile_server.or(helper.unstable_tile_server),
            #[cfg(feature = "unstable-msc4143")]
            rtc_foci: helper.stable_rtc_foci.or(helper.unstable_rtc_foci).unwrap_or_default(),
        })
    }
}

impl Serialize for ResponseBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap as _;

        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("m.homeserver", &self.homeserver)?;

        if let Some(identity_server) = &self.identity_server {
            map.serialize_entry("m.identity_server", identity_server)?;
        }

        // Only the unstable name is written, which is what the `rename` on the
        // field used to do: a reader that only knows the stable name is reading
        // a document the MSC has not stabilised yet either.
        #[cfg(feature = "unstable-msc3488")]
        if let Some(tile_server) = &self.tile_server {
            map.serialize_entry("org.matrix.msc3488.tile_server", tile_server)?;
        }

        #[cfg(feature = "unstable-msc4143")]
        if !self.rtc_foci.is_empty() {
            map.serialize_entry("org.matrix.msc4143.rtc_foci", &self.rtc_foci)?;
        }

        map.end()
    }
}

impl Request {
    /// Creates an empty `Request`.
    pub fn new() -> Self {
        Self {}
    }
}

impl Response {
    /// Creates a new `Response` with the given `HomeserverInfo`.
    pub fn new(homeserver: HomeserverInfo) -> Self {
        Self {
            homeserver,
            identity_server: None,
            #[cfg(feature = "unstable-msc3488")]
            tile_server: None,
            #[cfg(feature = "unstable-msc4143")]
            rtc_foci: Default::default(),
        }
    }
}

/// Information about a discovered homeserver.
#[derive(Clone, Debug, Deserialize, Hash, Serialize, PartialEq, Eq)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct HomeserverInfo {
    /// The base URL for the homeserver for client-server connections.
    pub base_url: String,
}

impl HomeserverInfo {
    /// Creates a new `HomeserverInfo` with the given `base_url`.
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

/// Information about a discovered identity server.
#[derive(Clone, Debug, Deserialize, Hash, Serialize, PartialEq, Eq)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct IdentityServerInfo {
    /// The base URL for the identity server for client-server connections.
    pub base_url: String,
}

impl IdentityServerInfo {
    /// Creates an `IdentityServerInfo` with the given `base_url`.
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

/// Information about a discovered map tile server.
#[cfg(feature = "unstable-msc3488")]
#[derive(Clone, Debug, Deserialize, Hash, Serialize, PartialEq, Eq)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
pub struct TileServerInfo {
    /// The URL of a map tile server's `style.json` file.
    ///
    /// See the [Mapbox Style Specification](https://docs.mapbox.com/mapbox-gl-js/style-spec/) for more details.
    pub map_style_url: String,
}

#[cfg(feature = "unstable-msc3488")]
impl TileServerInfo {
    /// Creates a `TileServerInfo` with the given map style URL.
    pub fn new(map_style_url: String) -> Self {
        Self { map_style_url }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_value as from_json_value, json, to_value as to_json_value};

    use super::ResponseBody;

    #[test]
    fn test_a_well_known_file_with_only_the_stable_names_is_read() {
        let body: ResponseBody = from_json_value(json!({
            "m.homeserver": { "base_url": "https://matrix.example.org" },
            "m.rtc_foci": [{ "type": "livekit", "livekit_service_url": "https://livekit.example.org" }],
        }))
        .unwrap();

        assert_eq!(body.homeserver.base_url, "https://matrix.example.org");
        #[cfg(feature = "unstable-msc4143")]
        assert_eq!(body.rtc_foci.len(), 1);
    }

    #[test]
    fn test_a_well_known_file_with_only_the_unstable_names_is_read() {
        let body: ResponseBody = from_json_value(json!({
            "m.homeserver": { "base_url": "https://matrix.example.org" },
            "org.matrix.msc4143.rtc_foci": [
                { "type": "livekit", "livekit_service_url": "https://livekit.example.org" },
            ],
        }))
        .unwrap();

        #[cfg(feature = "unstable-msc4143")]
        assert_eq!(body.rtc_foci.len(), 1);
        let _ = body;
    }

    /// A server listing a value under both its stable and its unstable name
    /// used to fail the whole document as a duplicate field, leaving the
    /// client with no discovery information at all.
    #[test]
    fn test_a_well_known_file_with_both_names_prefers_the_stable_one() {
        let body: ResponseBody = from_json_value(json!({
            "m.homeserver": { "base_url": "https://matrix.example.org" },
            "m.rtc_foci": [{ "type": "livekit", "livekit_service_url": "https://stable.example.org" }],
            "org.matrix.msc4143.rtc_foci": [
                { "type": "livekit", "livekit_service_url": "https://unstable.example.org" },
                { "type": "livekit", "livekit_service_url": "https://unstable2.example.org" },
            ],
        }))
        .unwrap();

        assert_eq!(body.homeserver.base_url, "https://matrix.example.org");
        #[cfg(feature = "unstable-msc4143")]
        assert_eq!(body.rtc_foci.len(), 1);
    }

    #[cfg(feature = "unstable-msc3488")]
    #[test]
    fn test_a_tile_server_under_both_names_prefers_the_stable_one() {
        let body: ResponseBody = from_json_value(json!({
            "m.homeserver": { "base_url": "https://matrix.example.org" },
            "m.tile_server": { "map_style_url": "https://stable.example.org/style.json" },
            "org.matrix.msc3488.tile_server": {
                "map_style_url": "https://unstable.example.org/style.json",
            },
        }))
        .unwrap();

        assert_eq!(
            body.tile_server.unwrap().map_style_url,
            "https://stable.example.org/style.json"
        );
    }

    #[test]
    fn test_the_body_serializes_back_to_the_names_it_advertises() {
        let body: ResponseBody = from_json_value(json!({
            "m.homeserver": { "base_url": "https://matrix.example.org" },
            "m.identity_server": { "base_url": "https://id.example.org" },
        }))
        .unwrap();

        // Absent optional fields are not written back as nulls or empty lists.
        assert_eq!(
            to_json_value(&body).unwrap(),
            json!({
                "m.homeserver": { "base_url": "https://matrix.example.org" },
                "m.identity_server": { "base_url": "https://id.example.org" },
            })
        );
    }
}
