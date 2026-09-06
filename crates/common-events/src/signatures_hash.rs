//! Types for the [MSC3834] to-device events that let a user's devices check
//! that the homeserver is not dropping their cross-signing signatures.
//!
//! A device may ask any of the user's other devices, with
//! `m.signatures_hash_request`, for a hash of the complete set of cross-signing
//! signatures that device knows about; the other device answers with
//! `m.signatures_hash`. Comparing the two tells a device whether the server has
//! been withholding signatures from it, which is how a malicious homeserver
//! would otherwise defeat opportunistic key pinning.
//!
//! [MSC3834]: https://github.com/matrix-org/matrix-spec-proposals/pull/3834

use harana_matrix_macros::EventContent;
use serde::{Deserialize, Serialize};

/// The content of an `m.signatures_hash_request` event.
///
/// Asks another of our own devices for the hash of the cross-signing signatures
/// it knows about. The event has no fields.
///
/// It must be encrypted as an `m.room.encrypted` event, then sent as a
/// to-device event.
#[derive(Clone, Debug, Default, Deserialize, Serialize, EventContent)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
#[ruma_event(type = "org.matrix.msc3834.v1.signatures_hash_request", kind = ToDevice)]
pub struct ToDeviceSignaturesHashRequestEventContent {}

impl ToDeviceSignaturesHashRequestEventContent {
    /// Creates a new `ToDeviceSignaturesHashRequestEventContent`.
    pub fn new() -> Self {
        Self {}
    }
}

/// The content of an `m.signatures_hash` event.
///
/// Answers an [`m.signatures_hash_request`] with a hash of the cross-signing
/// signatures this device knows about.
///
/// It must be encrypted as an `m.room.encrypted` event, then sent as a
/// to-device event.
///
/// [`m.signatures_hash_request`]: ToDeviceSignaturesHashRequestEventContent
#[derive(Clone, Debug, Deserialize, Serialize, EventContent)]
#[cfg_attr(not(ruma_unstable_exhaustive_types), non_exhaustive)]
#[ruma_event(type = "org.matrix.msc3834.v1.signatures_hash", kind = ToDevice)]
pub struct ToDeviceSignaturesHashEventContent {
    /// The SHA-256 hash of the canonical JSON encoding of the complete set of
    /// cross-signing signatures known to the sending device.
    ///
    /// The set is encoded in the shape the `/keys/signatures/upload` endpoint
    /// takes - a map from user ID, to key ID, to the signed JSON object for
    /// that key - with any `unsigned` fields left out.
    pub sha256: String,
}

impl ToDeviceSignaturesHashEventContent {
    /// Creates a new `ToDeviceSignaturesHashEventContent` with the given hash.
    pub fn new(sha256: String) -> Self {
        Self { sha256 }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_value as from_json_value, json, to_value as to_json_value};

    use super::{ToDeviceSignaturesHashEventContent, ToDeviceSignaturesHashRequestEventContent};

    #[test]
    fn test_signatures_hash_request_serializes_to_an_empty_object() {
        let content = ToDeviceSignaturesHashRequestEventContent::new();

        assert_eq!(to_json_value(&content).unwrap(), json!({}));
    }

    #[test]
    fn test_signatures_hash_round_trips() {
        let content = ToDeviceSignaturesHashEventContent::new("abcdef".to_owned());
        let json = to_json_value(&content).unwrap();

        assert_eq!(json, json!({ "sha256": "abcdef" }));

        let parsed: ToDeviceSignaturesHashEventContent = from_json_value(json).unwrap();
        assert_eq!(parsed.sha256, "abcdef");
    }
}
