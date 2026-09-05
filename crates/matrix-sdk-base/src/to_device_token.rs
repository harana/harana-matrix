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

//! The stream token of the to-device extension of [MSC4186] sliding sync.
//!
//! The crypto store holds a single token slot, which the sliding sync
//! to-device extension resumes from. Sync v2 used to write its own `next_batch`
//! into that same slot, so a client upgrading an existing store from sync v2 to
//! sliding sync would hand the server a sync v2 token where a to-device stream
//! token is expected, and every sync would fail until the store was deleted.
//!
//! To keep the two apart, tokens written by the sliding sync code path are
//! tagged with a marker, and only tagged tokens are resumed from. An untagged
//! value — a sync v2 `next_batch`, or a token written before this marker
//! existed — is ignored, and the to-device extension starts without a `since`.
//! The server then replays the to-device events it hasn't seen acknowledged,
//! which is what an initial to-device sync does anyway.
//!
//! [MSC4186]: https://github.com/matrix-org/matrix-spec-proposals/pull/4186

/// The marker prefixed to a to-device stream token that comes from a sliding
/// sync response.
///
/// Be careful: this ends up in a persisted value; changing it invalidates every
/// stored token.
const MARKER: &str = "msc4186:";

/// Tag a to-device stream token received in a sliding sync response, so that it
/// can be told apart from a sync v2 `next_batch` in the store.
pub fn tag(token: &str) -> String {
    format!("{MARKER}{token}")
}

/// Get back the to-device stream token from a stored value, if that value was
/// written by the sliding sync code path.
///
/// Returns `None` for a value that isn't tagged, which must not be sent to the
/// server as a to-device `since`.
pub fn untag(stored_value: &str) -> Option<&str> {
    stored_value.strip_prefix(MARKER)
}

#[cfg(test)]
mod tests {
    use super::{tag, untag};

    #[test]
    fn test_tagged_token_round_trips() {
        assert_eq!(untag(&tag("12345")), Some("12345"));
        assert_eq!(untag(&tag("")), Some(""));
    }

    #[test]
    fn test_untagged_token_is_rejected() {
        // A sync v2 `next_batch`, or a token stored before the marker existed.
        assert_eq!(untag("s72594_4483_1934"), None);
        assert_eq!(untag("12345"), None);
    }
}
