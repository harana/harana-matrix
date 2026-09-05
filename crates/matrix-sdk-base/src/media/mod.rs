// Copyright 2025 Kévin Commaille
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

//! Media store and common types for [media content](https://spec.matrix.org/latest/client-server-api/#content-repository).

pub mod store;

use std::hash::{Hash, Hasher};

use ruma::{
    MxcUri, UInt,
    api::client::media::get_content_thumbnail::v3::Method,
    events::{
        room::{
            MediaSource,
            message::{
                AudioMessageEventContent, FileMessageEventContent, ImageMessageEventContent,
                LocationMessageEventContent, VideoMessageEventContent,
            },
        },
        sticker::StickerEventContent,
    },
};
use serde::{Deserialize, Serialize};

const UNIQUE_SEPARATOR: &str = "_";

/// A trait to uniquely identify values of the same type.
pub trait UniqueKey {
    /// A string that uniquely identifies `Self` compared to other values of
    /// the same type.
    fn unique_key(&self) -> String;
}

/// The requested format of a media file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MediaFormat {
    /// The file that was uploaded.
    File,

    /// A thumbnail of the file that was uploaded.
    Thumbnail(MediaThumbnailSettings),
}

impl UniqueKey for MediaFormat {
    fn unique_key(&self) -> String {
        match self {
            Self::File => "file".into(),
            Self::Thumbnail(settings) => settings.unique_key(),
        }
    }
}

/// The desired settings of a media thumbnail.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MediaThumbnailSettings {
    /// The desired resizing method.
    pub method: Method,

    /// The desired width of the thumbnail. The actual thumbnail may not match
    /// the size specified.
    pub width: UInt,

    /// The desired height of the thumbnail. The actual thumbnail may not match
    /// the size specified.
    pub height: UInt,

    /// If we want to request an animated thumbnail from the homeserver.
    ///
    /// If it is `true`, the server should return an animated thumbnail if
    /// the media supports it.
    ///
    /// Defaults to `false`.
    pub animated: bool,
}

impl MediaThumbnailSettings {
    /// Constructs a new `MediaThumbnailSettings` with the given method, width
    /// and height.
    ///
    /// Requests a non-animated thumbnail by default.
    pub fn with_method(method: Method, width: UInt, height: UInt) -> Self {
        Self { method, width, height, animated: false }
    }

    /// Constructs a new `MediaThumbnailSettings` with the given width and
    /// height.
    ///
    /// Requests scaling, and a non-animated thumbnail.
    pub fn new(width: UInt, height: UInt) -> Self {
        Self { method: Method::Scale, width, height, animated: false }
    }
}

// `Method` is a `StringEnum` and doesn't implement `Hash`, so this can't be
// derived; hash the string representation instead, which is what its
// `PartialEq` compares.
impl Hash for MediaThumbnailSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.as_str().hash(state);
        self.width.hash(state);
        self.height.hash(state);
        self.animated.hash(state);
    }
}

impl UniqueKey for MediaThumbnailSettings {
    fn unique_key(&self) -> String {
        let mut key = format!("{}{UNIQUE_SEPARATOR}{}x{}", self.method, self.width, self.height);

        if self.animated {
            key.push_str(UNIQUE_SEPARATOR);
            key.push_str("animated");
        }

        key
    }
}

impl UniqueKey for MediaSource {
    fn unique_key(&self) -> String {
        match self {
            Self::Plain(uri) => uri.to_string(),
            Self::Encrypted(file) => file.url.to_string(),
        }
    }
}

/// Parameters for a request for retrieve media data.
///
/// This is used as a key in the media cache too.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaRequestParameters {
    /// The source of the media file.
    pub source: MediaSource,

    /// The requested format of the media data.
    pub format: MediaFormat,
}

impl MediaRequestParameters {
    /// Get the [`MxcUri`] from `Self`.
    pub fn uri(&self) -> &MxcUri {
        match &self.source {
            MediaSource::Plain(url) => url.as_ref(),
            MediaSource::Encrypted(file) => file.url.as_ref(),
        }
    }
}

impl UniqueKey for MediaRequestParameters {
    fn unique_key(&self) -> String {
        format!("{}{UNIQUE_SEPARATOR}{}", self.source.unique_key(), self.format.unique_key())
    }
}

/// Trait for media event content.
pub trait MediaEventContent {
    /// Get the source of the file for `Self`.
    ///
    /// Returns `None` if `Self` has no file.
    fn source(&self) -> Option<MediaSource>;

    /// Get the source of the thumbnail for `Self`.
    ///
    /// Returns `None` if `Self` has no thumbnail.
    fn thumbnail_source(&self) -> Option<MediaSource>;
}

impl MediaEventContent for StickerEventContent {
    fn source(&self) -> Option<MediaSource> {
        Some(MediaSource::from(self.source.clone()))
    }

    fn thumbnail_source(&self) -> Option<MediaSource> {
        None
    }
}

impl MediaEventContent for AudioMessageEventContent {
    fn source(&self) -> Option<MediaSource> {
        Some(self.source.clone())
    }

    fn thumbnail_source(&self) -> Option<MediaSource> {
        None
    }
}

impl MediaEventContent for FileMessageEventContent {
    fn source(&self) -> Option<MediaSource> {
        Some(self.source.clone())
    }

    fn thumbnail_source(&self) -> Option<MediaSource> {
        self.info.as_ref()?.thumbnail_source.clone()
    }
}

impl MediaEventContent for ImageMessageEventContent {
    fn source(&self) -> Option<MediaSource> {
        Some(self.source.clone())
    }

    fn thumbnail_source(&self) -> Option<MediaSource> {
        self.info
            .as_ref()
            .and_then(|info| info.thumbnail_source.clone())
            .or_else(|| Some(self.source.clone()))
    }
}

impl MediaEventContent for VideoMessageEventContent {
    fn source(&self) -> Option<MediaSource> {
        Some(self.source.clone())
    }

    fn thumbnail_source(&self) -> Option<MediaSource> {
        self.info
            .as_ref()
            .and_then(|info| info.thumbnail_source.clone())
            .or_else(|| Some(self.source.clone()))
    }
}

impl MediaEventContent for LocationMessageEventContent {
    fn source(&self) -> Option<MediaSource> {
        None
    }

    fn thumbnail_source(&self) -> Option<MediaSource> {
        self.info.as_ref()?.thumbnail_source.clone()
    }
}

#[cfg(test)]
mod tests {
    use assert_matches2::assert_let;
    use ruma::{events::room::ImageInfo, mxc_uri, owned_mxc_uri, uint};
    use serde_json::json;

    use super::*;

    #[test]
    fn test_media_format_can_key_a_map() {
        use std::collections::{BTreeMap, HashMap};

        let file = MediaFormat::File;
        let small = MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(32), uint!(32)));
        let large = MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(64), uint!(64)));
        let cropped = MediaFormat::Thumbnail(MediaThumbnailSettings::with_method(
            Method::Crop,
            uint!(32),
            uint!(32),
        ));

        // Equality distinguishes every setting.
        assert_eq!(
            small,
            MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(32), uint!(32)))
        );
        assert_ne!(small, large);
        assert_ne!(small, cropped);
        assert_ne!(small, file);

        // `Ord` makes it usable as a `BTreeMap` key...
        let mut ordered = BTreeMap::new();
        ordered.insert(file, "file");
        ordered.insert(small.clone(), "small");
        ordered.insert(large.clone(), "large");

        assert_eq!(ordered.get(&small), Some(&"small"));
        assert_eq!(ordered.len(), 3);

        // ... and `Hash` as a `HashMap` one. `Method` is a string enum without a
        // `Hash` implementation, so this is hand-written and worth checking.
        let mut hashed = HashMap::new();
        hashed.insert(small.clone(), "small");
        hashed.insert(cropped.clone(), "cropped");

        assert_eq!(hashed.get(&small), Some(&"small"));
        assert_eq!(hashed.get(&cropped), Some(&"cropped"));
        assert_eq!(hashed.get(&large), None);

        // Inserting an equal value replaces rather than duplicates.
        hashed.insert(
            MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(32), uint!(32))),
            "again",
        );
        assert_eq!(hashed.len(), 2);
        assert_eq!(hashed.get(&small), Some(&"again"));
    }

    #[test]
    fn test_media_request_url() {
        let mxc_uri = mxc_uri!("mxc://homeserver/media");

        let plain = MediaRequestParameters {
            source: MediaSource::Plain(mxc_uri.to_owned()),
            format: MediaFormat::File,
        };

        assert_eq!(plain.uri(), mxc_uri);

        let file = MediaRequestParameters {
            source: MediaSource::Encrypted(Box::new(
                serde_json::from_value(json!({
                    "url": mxc_uri,
                    "key": {
                        "kty": "oct",
                        "key_ops": ["encrypt", "decrypt"],
                        "alg": "A256CTR",
                        "k": "b50ACIv6LMn9AfMCFD1POJI_UAFWIclxAN1kWrEO2X8",
                        "ext": true,
                    },
                    "iv": "AK1wyzigZtQAAAABAAAAKK",
                    "hashes": {
                        "sha256": "/NogKqW5bz/m8xHgFiH5haFGjCNVmUIPLzfvOhHdrxY",
                    },
                    "v": "v2",
                }))
                .unwrap(),
            )),
            format: MediaFormat::File,
        };

        assert_eq!(file.uri(), mxc_uri);
    }
    /// The unique key of a media request must distinguish the file from its
    /// thumbnails, and the thumbnails from each other.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#thumbnails>.
    #[test]
    fn test_media_format_unique_keys() {
        assert_eq!(MediaFormat::File.unique_key(), "file");

        let scaled = MediaThumbnailSettings::new(uint!(100), uint!(50));
        assert_eq!(scaled.unique_key(), "scale_100x50");

        let cropped = MediaThumbnailSettings::with_method(Method::Crop, uint!(100), uint!(50));
        assert_eq!(cropped.unique_key(), "crop_100x50");

        // The dimensions are part of the key…
        let bigger = MediaThumbnailSettings::new(uint!(200), uint!(50));
        assert_ne!(scaled.unique_key(), bigger.unique_key());

        // …and so is the animated flag.
        let mut animated = MediaThumbnailSettings::new(uint!(100), uint!(50));
        animated.animated = true;
        assert_eq!(animated.unique_key(), "scale_100x50_animated");
    }

    /// The unique key of a media request combines the source and the format,
    /// so that the file and its thumbnails are cached separately.
    #[test]
    fn test_media_request_unique_key() {
        let source = MediaSource::Plain(owned_mxc_uri!("mxc://homeserver/media"));

        let file = MediaRequestParameters { source: source.clone(), format: MediaFormat::File };
        assert_eq!(file.unique_key(), "mxc://homeserver/media_file");

        let thumbnail = MediaRequestParameters {
            source,
            format: MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(100), uint!(50))),
        };
        assert_eq!(thumbnail.unique_key(), "mxc://homeserver/media_scale_100x50");

        assert_ne!(file.unique_key(), thumbnail.unique_key());
    }

    /// The unique key of an encrypted source is its URL, so that a file keeps
    /// the same cache key whether it is encrypted or not.
    #[test]
    fn test_encrypted_source_unique_key() {
        let mxc_uri = mxc_uri!("mxc://homeserver/media");
        let encrypted = MediaSource::Encrypted(Box::new(
            serde_json::from_value(json!({
                "url": mxc_uri,
                "key": {
                    "kty": "oct",
                    "key_ops": ["encrypt", "decrypt"],
                    "alg": "A256CTR",
                    "k": "b50ACIv6LMn9AfMCFD1POJI_UAFWIclxAN1kWrEO2X8",
                    "ext": true,
                },
                "iv": "AK1wyzigZtQAAAABAAAAKK",
                "hashes": {
                    "sha256": "/NogKqW5bz/m8xHgFiH5haFGjCNVmUIPLzfvOhHdrxY",
                },
                "v": "v2",
            }))
            .unwrap(),
        ));

        assert_eq!(encrypted.unique_key(), mxc_uri.to_string());
        assert_eq!(MediaSource::Plain(mxc_uri.to_owned()).unique_key(), encrypted.unique_key());
    }

    /// An `m.image` message has a file, and falls back to the file itself when
    /// it has no dedicated thumbnail.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mimage>.
    #[test]
    fn test_image_message_media_sources() {
        let file = owned_mxc_uri!("mxc://homeserver/image");
        let thumbnail = owned_mxc_uri!("mxc://homeserver/thumbnail");

        let mut content = ImageMessageEventContent::plain("image.png".to_owned(), file.clone());

        assert_let!(Some(MediaSource::Plain(uri)) = content.source());
        assert_eq!(uri, file);

        // No thumbnail: the file itself is used.
        assert_let!(Some(MediaSource::Plain(uri)) = content.thumbnail_source());
        assert_eq!(uri, file);

        // With a thumbnail: the thumbnail is used.
        let mut info = ImageInfo::new();
        info.thumbnail_source = Some(MediaSource::Plain(thumbnail.clone()));
        content.info = Some(Box::new(info));

        assert_let!(Some(MediaSource::Plain(uri)) = content.thumbnail_source());
        assert_eq!(uri, thumbnail);
    }

    /// An `m.video` message behaves like an `m.image` one: it falls back to
    /// the file when it has no thumbnail.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mvideo>.
    #[test]
    fn test_video_message_media_sources() {
        let file = owned_mxc_uri!("mxc://homeserver/video");

        let content = VideoMessageEventContent::plain("video.mp4".to_owned(), file.clone());

        assert_let!(Some(MediaSource::Plain(uri)) = content.source());
        assert_eq!(uri, file);
        assert_let!(Some(MediaSource::Plain(uri)) = content.thumbnail_source());
        assert_eq!(uri, file);
    }

    /// An `m.file` message has no thumbnail unless one is explicitly given;
    /// the file is not a valid thumbnail of itself.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mfile>.
    #[test]
    fn test_file_message_media_sources() {
        let file = owned_mxc_uri!("mxc://homeserver/document");
        let thumbnail = owned_mxc_uri!("mxc://homeserver/thumbnail");

        let mut content = FileMessageEventContent::plain("document.pdf".to_owned(), file.clone());

        assert_let!(Some(MediaSource::Plain(uri)) = content.source());
        assert_eq!(uri, file);
        assert!(content.thumbnail_source().is_none());

        let mut info = ruma::events::room::message::FileInfo::new();
        info.thumbnail_source = Some(MediaSource::Plain(thumbnail.clone()));
        content.info = Some(Box::new(info));

        assert_let!(Some(MediaSource::Plain(uri)) = content.thumbnail_source());
        assert_eq!(uri, thumbnail);
    }

    /// An `m.audio` message has a file but never a thumbnail.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#maudio>.
    #[test]
    fn test_audio_message_media_sources() {
        let file = owned_mxc_uri!("mxc://homeserver/audio");

        let content = AudioMessageEventContent::plain("audio.ogg".to_owned(), file.clone());

        assert_let!(Some(MediaSource::Plain(uri)) = content.source());
        assert_eq!(uri, file);
        assert!(content.thumbnail_source().is_none());
    }

    /// An `m.location` message has no file, but it can have a thumbnail.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#mlocation>.
    #[test]
    fn test_location_message_media_sources() {
        let thumbnail = owned_mxc_uri!("mxc://homeserver/thumbnail");

        let mut content = LocationMessageEventContent::new(
            "Alice was here".to_owned(),
            "geo:51.5008,0.1247".to_owned(),
        );

        assert!(content.source().is_none());
        assert!(content.thumbnail_source().is_none());

        let mut info = ruma::events::room::message::LocationInfo::new();
        info.thumbnail_source = Some(MediaSource::Plain(thumbnail.clone()));
        content.info = Some(Box::new(info));

        assert!(content.source().is_none());
        assert_let!(Some(MediaSource::Plain(uri)) = content.thumbnail_source());
        assert_eq!(uri, thumbnail);
    }

    /// An `m.sticker` event has a file but no thumbnail.
    ///
    /// See <https://spec.matrix.org/v1.16/client-server-api/#msticker>.
    #[test]
    fn test_sticker_media_sources() {
        let file = owned_mxc_uri!("mxc://homeserver/sticker");

        let content =
            StickerEventContent::new("sticker".to_owned(), ImageInfo::new(), file.clone());

        assert_let!(Some(MediaSource::Plain(uri)) = content.source());
        assert_eq!(uri, file);
        assert!(content.thumbnail_source().is_none());
    }
}
