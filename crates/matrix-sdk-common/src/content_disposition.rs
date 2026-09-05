// Copyright 2025 Tuwunel Contributors
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
// Ported from tuwunel `src/core/utils/content_disposition.rs`.

//! HTTP content-disposition selection and filename sanitization.
//!
//! Media types are checked against the safe-inline list before selecting a
//! disposition. Optional filenames are sanitized before header construction.
//!
//! A client that renders or saves downloaded media needs the same decision a
//! homeserver makes when it serves that media: content the browser may render
//! inline is limited to the [MSC2702] list, and everything else is an
//! attachment. A filename taken from a remote `Content-Disposition` is attacker
//! controlled and is sanitized before use.
//!
//! [MSC2702]: https://github.com/matrix-org/matrix-spec-proposals/pull/2702

use ruma::http_headers::{ContentDisposition, ContentDispositionType};
use sanitize_filename::{Options, sanitize_with_options};
use tracing::debug;

/// Media types a client may render inline, as defined by MSC2702.
const ALLOWED_INLINE_CONTENT_TYPES: [&str; 26] = [
    // keep sorted
    "application/json",
    "application/ld+json",
    "audio/aac",
    "audio/flac",
    "audio/mp4",
    "audio/mpeg",
    "audio/ogg",
    "audio/wav",
    "audio/wave",
    "audio/webm",
    "audio/x-flac",
    "audio/x-pn-wav",
    "audio/x-wav",
    "image/apng",
    "image/avif",
    "image/gif",
    "image/jpeg",
    "image/png",
    "image/webp",
    "text/css",
    "text/csv",
    "text/plain",
    "video/mp4",
    "video/ogg",
    "video/quicktime",
    "video/webm",
];

/// Returns a Content-Disposition of `attachment` or `inline`, depending on the
/// Content-Type against the MSC2702 list of safe inline Content-Types
/// (`ALLOWED_INLINE_CONTENT_TYPES`).
///
/// An absent Content-Type is treated as an attachment.
#[must_use]
pub fn content_disposition_type(content_type: Option<&str>) -> ContentDispositionType {
    let Some(content_type) = content_type else {
        debug!("No Content-Type was given, assuming attachment for Content-Disposition");
        return ContentDispositionType::Attachment;
    };

    debug_assert!(
        ALLOWED_INLINE_CONTENT_TYPES.is_sorted(),
        "ALLOWED_INLINE_CONTENT_TYPES is not sorted"
    );

    let essence = content_type_essence(content_type);

    // The list is lowercase and ordered bytewise, so folding the needle's case as
    // it is compared searches the same order without copying it.
    let allowed = ALLOWED_INLINE_CONTENT_TYPES
        .binary_search_by(|allowed| {
            allowed.bytes().cmp(essence.bytes().map(|byte| byte.to_ascii_lowercase()))
        })
        .is_ok();

    if allowed { ContentDispositionType::Inline } else { ContentDispositionType::Attachment }
}

/// Whether a Content-Type names the given media type.
///
/// Media types are case-insensitive per RFC 9110 section 8.3.1, so the
/// comparison folds case rather than requiring an exact spelling.
#[inline]
#[must_use]
pub fn content_type_is(content_type: Option<&str>, essence: &str) -> bool {
    content_type.is_some_and(|content_type| {
        content_type_essence(content_type).eq_ignore_ascii_case(essence)
    })
}

/// The media type of a Content-Type, without its parameters.
///
/// A header value is `type/subtype` followed by optional `;` parameters, and
/// callers deciding what a body is must weigh only the former: a parameter that
/// merely contains a media type does not make the body that type.
#[inline]
#[must_use]
pub fn content_type_essence(content_type: &str) -> &str {
    content_type.split(';').next().unwrap_or(content_type).trim()
}

/// Sanitizes a filename for use in a Content-Disposition header.
///
/// Path separators, traversal sequences, and control characters are removed.
/// The name is not truncated.
#[tracing::instrument(level = "debug")]
pub fn sanitize_filename(filename: &str) -> String {
    sanitize_with_options(filename, Options { truncate: false, ..Default::default() })
}

/// Creates the final Content-Disposition based on whether the filename exists
/// or not, or if a requested filename was specified (media download with
/// filename).
///
/// If a filename is present:
/// `Content-Disposition: attachment/inline; filename=filename.ext`
///
/// Otherwise: `Content-Disposition: attachment/inline`
///
/// An explicit `filename` wins over one carried by `content_disposition`;
/// either is sanitized before it reaches the header.
pub fn make_content_disposition(
    content_disposition: Option<&ContentDisposition>,
    content_type: Option<&str>,
    filename: Option<&str>,
) -> ContentDisposition {
    ContentDisposition::new(content_disposition_type(content_type)).with_filename(
        filename
            .or_else(|| {
                content_disposition
                    .and_then(|content_disposition| content_disposition.filename.as_deref())
            })
            .map(sanitize_filename),
    )
}

#[cfg(test)]
mod tests {
    use ruma::http_headers::{ContentDisposition, ContentDispositionType};

    use super::{
        content_disposition_type, content_type_essence, content_type_is, make_content_disposition,
        sanitize_filename,
    };

    #[test]
    fn test_inline_types_are_matched_case_insensitively() {
        for content_type in ["image/png", "Image/PNG", "image/png; charset=binary"] {
            assert_eq!(
                content_disposition_type(Some(content_type)),
                ContentDispositionType::Inline,
                "{content_type} should be inline"
            );
        }
    }

    #[test]
    fn test_everything_else_is_an_attachment() {
        for content_type in [None, Some("text/html"), Some("application/octet-stream")] {
            assert_eq!(
                content_disposition_type(content_type),
                ContentDispositionType::Attachment,
                "{content_type:?} should be an attachment"
            );
        }
    }

    #[test]
    fn test_content_type_essence_drops_parameters() {
        assert_eq!(content_type_essence("text/html; charset=utf-8"), "text/html");
        assert_eq!(content_type_essence(" text/html "), "text/html");
        assert_eq!(content_type_essence("text/html"), "text/html");
    }

    #[test]
    fn test_content_type_is_matches_any_case() {
        for content_type in ["text/html", "Text/HTML", "TEXT/HTML", "text/HTML; charset=utf-8"] {
            assert!(content_type_is(Some(content_type), "text/html"), "{content_type} is html");
        }
    }

    #[test]
    fn test_content_type_is_rejects_other_types() {
        for content_type in ["application/json; x=text/html", "text/plain", "application/xhtml+xml"]
        {
            assert!(
                !content_type_is(Some(content_type), "text/html"),
                "{content_type} is not html"
            );
        }
    }

    #[test]
    fn test_traversal_is_stripped_from_filenames() {
        // Separators are what make a name a path, so their removal is what stops
        // the traversal. The dots themselves are left in the name.
        let sanitized = sanitize_filename("../../../etc/passwd");
        assert!(!sanitized.contains('/'), "{sanitized} still contains a path separator");
        assert!(!sanitized.contains('\\'), "{sanitized} still contains a path separator");

        let sanitized = sanitize_filename("with\r\nnewlines\u{0}.png");
        assert!(!sanitized.contains('\n'), "{sanitized:?} still contains a control character");
        assert!(!sanitized.contains('\u{0}'), "{sanitized:?} still contains a control character");
    }

    #[test]
    fn test_an_explicit_filename_wins_over_the_header() {
        let remote = ContentDisposition::new(ContentDispositionType::Inline)
            .with_filename(Some("remote.png".to_owned()));

        let disposition =
            make_content_disposition(Some(&remote), Some("image/png"), Some("local.png"));

        assert_eq!(disposition.disposition_type, ContentDispositionType::Inline);
        assert_eq!(disposition.filename.as_deref(), Some("local.png"));

        let disposition = make_content_disposition(Some(&remote), Some("text/html"), None);

        assert_eq!(disposition.disposition_type, ContentDispositionType::Attachment);
        assert_eq!(disposition.filename.as_deref(), Some("remote.png"));
    }
}
