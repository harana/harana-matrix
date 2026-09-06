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
// Ported from tuwunel `src/service/resolver/well_known.rs`.

//! The server delegation document.

use serde::Deserialize;

/// How much of a `.well-known` response is worth reading.
///
/// The document holds one short string, so a response beyond this is either
/// broken or hostile. Tuwunel caps its read here for the same reason.
pub const WELL_KNOWN_MAX_BYTES: usize = 12288;

/// The delegation document's shape.
#[derive(Debug, Deserialize)]
struct WellKnown {
    #[serde(rename = "m.server")]
    server: Option<String>,
}

/// The URL a server's delegation document is fetched from.
#[must_use]
pub fn well_known_url(server_name: &str) -> String {
    format!("https://{server_name}/.well-known/matrix/server")
}

/// Reads the delegated server name out of a delegation document.
///
/// Returns `None` for a document that is not JSON, carries no `m.server`, or
/// carries an empty one. A malformed document is not an error: the
/// specification's ladder simply continues to the SRV step.
#[must_use]
pub fn parse_well_known(body: &str) -> Option<String> {
    let well_known: WellKnown = serde_json::from_str(body).ok()?;
    let server = well_known.server?;
    let server = server.trim();

    (!server.is_empty()).then(|| server.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{parse_well_known, well_known_url};

    #[test]
    fn test_the_url_is_the_specified_path() {
        assert_eq!(well_known_url("matrix.org"), "https://matrix.org/.well-known/matrix/server");
    }

    #[test]
    fn test_a_delegation_names_its_server() {
        assert_eq!(
            parse_well_known(r#"{"m.server": "matrix.matrix.org:443"}"#),
            Some("matrix.matrix.org:443".to_owned())
        );
        assert_eq!(
            parse_well_known(r#"{"m.server": " matrix.org "}"#),
            Some("matrix.org".to_owned())
        );
    }

    #[test]
    fn test_a_document_without_a_server_delegates_nothing() {
        assert_eq!(parse_well_known("{}"), None);
        assert_eq!(parse_well_known(r#"{"m.server": ""}"#), None);
        assert_eq!(parse_well_known(r#"{"m.server": "   "}"#), None);
        assert_eq!(parse_well_known("not json at all"), None);
        assert_eq!(parse_well_known(""), None);
    }

    #[test]
    fn test_unknown_fields_are_ignored() {
        assert_eq!(
            parse_well_known(r#"{"m.server": "matrix.org", "something.else": 42}"#),
            Some("matrix.org".to_owned())
        );
    }
}
