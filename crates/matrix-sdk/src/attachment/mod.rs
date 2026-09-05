// Copyright 2022 Kévin Commaille
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

//! Types and traits for attachments.

#[cfg(feature = "image-proc")]
mod blurhash;
mod exif;

use std::time::Duration;

use ruma::{
    OwnedTransactionId, UInt, assign,
    events::{
        Mentions,
        room::{
            ImageInfo, ThumbnailInfo,
            message::{AudioInfo, FileInfo, TextMessageEventContent, VideoInfo},
        },
    },
};

use crate::room::reply::Reply;

/// Base metadata about an image.
#[derive(Debug, Clone, Default)]
pub struct BaseImageInfo {
    /// The height of the image in pixels.
    pub height: Option<UInt>,
    /// The width of the image in pixels.
    pub width: Option<UInt>,
    /// The file size of the image in bytes.
    pub size: Option<UInt>,
    /// The [BlurHash](https://blurha.sh/) for this image.
    pub blurhash: Option<String>,
    /// Whether this image is animated.
    pub is_animated: Option<bool>,
}

/// Base metadata about a video.
#[derive(Debug, Clone, Default)]
pub struct BaseVideoInfo {
    /// The duration of the video.
    pub duration: Option<Duration>,
    /// The height of the video in pixels.
    pub height: Option<UInt>,
    /// The width of the video in pixels.
    pub width: Option<UInt>,
    /// The file size of the video in bytes.
    pub size: Option<UInt>,
    /// The [BlurHash](https://blurha.sh/) for this video.
    pub blurhash: Option<String>,
}

/// Base metadata about an audio clip.
#[derive(Debug, Clone, Default)]
pub struct BaseAudioInfo {
    /// The duration of the audio clip.
    pub duration: Option<Duration>,
    /// The file size of the audio clip in bytes.
    pub size: Option<UInt>,
    /// The waveform of the audio clip.
    ///
    /// Must only include values between 0 and 1.
    pub waveform: Option<Vec<f32>>,
}

/// Base metadata about a file.
#[derive(Debug, Clone, Default)]
pub struct BaseFileInfo {
    /// The size of the file in bytes.
    pub size: Option<UInt>,
}

/// Types of metadata for an attachment.
#[derive(Debug)]
pub enum AttachmentInfo {
    /// The metadata of an image.
    Image(BaseImageInfo),
    /// The metadata of a video.
    Video(BaseVideoInfo),
    /// The metadata of an audio clip.
    Audio(BaseAudioInfo),
    /// The metadata of a file.
    File(BaseFileInfo),
    /// The metadata of a voice message
    Voice(BaseAudioInfo),
}

impl From<AttachmentInfo> for ImageInfo {
    fn from(info: AttachmentInfo) -> Self {
        match info {
            AttachmentInfo::Image(info) => assign!(ImageInfo::new(), {
                height: info.height,
                width: info.width,
                size: info.size,
                blurhash: info.blurhash,
                is_animated: info.is_animated,
            }),
            _ => ImageInfo::new(),
        }
    }
}

impl From<AttachmentInfo> for VideoInfo {
    fn from(info: AttachmentInfo) -> Self {
        match info {
            AttachmentInfo::Video(info) => assign!(VideoInfo::new(), {
                duration: info.duration,
                height: info.height,
                width: info.width,
                size: info.size,
                blurhash: info.blurhash,
            }),
            _ => VideoInfo::new(),
        }
    }
}

impl From<AttachmentInfo> for AudioInfo {
    fn from(info: AttachmentInfo) -> Self {
        match info {
            AttachmentInfo::Audio(info) | AttachmentInfo::Voice(info) => {
                assign!(AudioInfo::new(), {
                    duration: info.duration,
                    size: info.size,
                })
            }
            _ => AudioInfo::new(),
        }
    }
}

impl From<AttachmentInfo> for FileInfo {
    fn from(info: AttachmentInfo) -> Self {
        match info {
            AttachmentInfo::File(info) => assign!(FileInfo::new(), {
                size: info.size,
            }),
            _ => FileInfo::new(),
        }
    }
}

/// A thumbnail to upload and send for an attachment.
#[derive(Debug)]
pub struct Thumbnail {
    /// The raw bytes of the thumbnail.
    pub data: Vec<u8>,
    /// The type of the thumbnail, this will be used as the content-type header.
    pub content_type: mime::Mime,
    /// The height of the thumbnail in pixels.
    pub height: UInt,
    /// The width of the thumbnail in pixels.
    pub width: UInt,
    /// The file size of the thumbnail in bytes.
    pub size: UInt,
}

impl Thumbnail {
    /// Convert this `Thumbnail` into a `(data, content_type, info)` tuple.
    pub fn into_parts(self) -> (Vec<u8>, mime::Mime, Box<ThumbnailInfo>) {
        let thumbnail_info = assign!(ThumbnailInfo::new(), {
            height: Some(self.height),
            width: Some(self.width),
            size: Some(self.size),
            mimetype: Some(self.content_type.to_string())
        });
        (self.data, self.content_type, Box::new(thumbnail_info))
    }
}

/// Configuration for sending an attachment.
#[derive(Debug, Default)]
pub struct AttachmentConfig {
    /// A fixed transaction id to be used for sending this attachment.
    ///
    /// Otherwise, a random one will be generated.
    pub txn_id: Option<OwnedTransactionId>,

    /// Type-specific metadata about the attachment.
    pub info: Option<AttachmentInfo>,

    /// An optional thumbnail to send with the attachment.
    pub thumbnail: Option<Thumbnail>,

    /// An optional caption for the attachment.
    pub caption: Option<TextMessageEventContent>,

    /// Intentional mentions to be included in the media event.
    pub mentions: Option<Mentions>,

    /// Reply parameters for the attachment (replied-to event and thread-related
    /// metadata).
    pub reply: Option<Reply>,

    /// Additional top-level fields to include in the media event's content.
    /// The event's own fields take precedence on conflicts.
    pub extra_content: Option<serde_json::Map<String, serde_json::Value>>,

    /// Whether to remove the metadata embedded in the image before uploading
    /// it.
    ///
    /// See [`AttachmentConfig::strip_exif`].
    pub strip_exif: bool,

    /// Whether to compute the [BlurHash] of the image before uploading it.
    ///
    /// See [`AttachmentConfig::generate_blurhash`].
    ///
    /// [BlurHash]: https://blurha.sh/
    pub generate_blurhash: bool,
}

impl AttachmentConfig {
    /// Create a new empty `AttachmentConfig`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the thumbnail to send.
    ///
    /// # Arguments
    ///
    /// * `thumbnail` - The thumbnail of the media. If the `content_type` does
    ///   not support it (e.g. audio clips), it is ignored.
    #[must_use]
    pub fn thumbnail(mut self, thumbnail: Option<Thumbnail>) -> Self {
        self.thumbnail = thumbnail;
        self
    }

    /// Set the transaction ID to send.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - A unique ID that can be attached to a `MessageEvent` held
    ///   in its unsigned field as `transaction_id`. If not given, one is
    ///   created for the message.
    #[must_use]
    pub fn txn_id(mut self, txn_id: OwnedTransactionId) -> Self {
        self.txn_id = Some(txn_id);
        self
    }

    /// Set the media metadata to send.
    ///
    /// # Arguments
    ///
    /// * `info` - The metadata of the media. If the `AttachmentInfo` type
    ///   doesn't match the `content_type`, it is ignored.
    #[must_use]
    pub fn info(mut self, info: AttachmentInfo) -> Self {
        self.info = Some(info);
        self
    }

    /// Set the optional caption.
    ///
    /// # Arguments
    ///
    /// * `caption` - The optional caption.
    pub fn caption(mut self, caption: Option<TextMessageEventContent>) -> Self {
        self.caption = caption;
        self
    }

    /// Set the mentions of the message.
    ///
    /// # Arguments
    ///
    /// * `mentions` - The mentions of the message.
    pub fn mentions(mut self, mentions: Option<Mentions>) -> Self {
        self.mentions = mentions;
        self
    }

    /// Set the reply information of the message.
    ///
    /// # Arguments
    ///
    /// * `reply` - The reply information of the message.
    pub fn reply(mut self, reply: Option<Reply>) -> Self {
        self.reply = reply;
        self
    }

    /// Remove the metadata embedded in the image before uploading it.
    ///
    /// A photo straight off a phone carries an Exif block holding the GPS
    /// coordinates and the wall-clock time of the shot, the device model and
    /// sometimes its serial number. With this set, that block, and the other
    /// metadata containers (XMP, IPTC, PNG text chunks, comments), are removed
    /// from the attachment and from its thumbnail before either is uploaded.
    ///
    /// The Exif `Orientation` tag is deliberately kept, so photos are still
    /// displayed the right way up. The pixels are never re-encoded, so this
    /// costs no image quality.
    ///
    /// This applies to JPEG, PNG and WebP images. Any other attachment is
    /// uploaded unchanged, including HEIC/HEIF photos, whose metadata this SDK
    /// cannot yet parse; a client handling those should strip them itself.
    ///
    /// Off by default, since the sender may well want to keep the metadata.
    #[must_use]
    pub fn strip_exif(mut self, strip_exif: bool) -> Self {
        self.strip_exif = strip_exif;
        self
    }

    /// Compute the [BlurHash] of the image before uploading it, and put it in
    /// the media event's content.
    ///
    /// A BlurHash is a short string describing the rough colours of an image.
    /// A receiving client can render it instantly, as a blurred placeholder,
    /// while the media itself is still downloading. It is sent in the clear
    /// even in an encrypted room, but at this resolution it reveals no more
    /// than a heavily blurred thumbnail would.
    ///
    /// For an image attachment the hash is computed from the image; for a
    /// video, from its thumbnail, since the SDK cannot decode video. A
    /// BlurHash already present in [`AttachmentConfig::info`] is left alone.
    ///
    /// This requires the `image-proc` feature, which pulls in an image
    /// decoder; without it, asking for a BlurHash logs a warning and does
    /// nothing.
    ///
    /// Off by default, since decoding the image costs time and memory.
    ///
    /// [BlurHash]: https://blurha.sh/
    #[must_use]
    pub fn generate_blurhash(mut self, generate_blurhash: bool) -> Self {
        self.generate_blurhash = generate_blurhash;
        self
    }

    /// Set additional top-level fields for the media event's content.
    ///
    /// # Arguments
    ///
    /// * `extra_content` - The additional fields.
    pub fn extra_content(
        mut self,
        extra_content: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        self.extra_content = extra_content;
        self
    }
}

/// Apply the preprocessing steps [`AttachmentConfig`] asks for to an
/// attachment and its thumbnail, before either is cached or uploaded.
///
/// The attachment info is updated in place, since a computed BlurHash belongs
/// in the media event's content.
///
/// This runs on a blocking thread: an attachment can be tens of megabytes, and
/// neither walking it nor decoding it must stall the caller's executor.
pub(crate) async fn preprocess(
    content_type: &mime::Mime,
    data: Vec<u8>,
    thumbnail: Option<Thumbnail>,
    config: &mut AttachmentConfig,
) -> (Vec<u8>, Option<Thumbnail>) {
    let strip_exif = config.strip_exif;
    let generate_blurhash = config.generate_blurhash;

    if !strip_exif && !generate_blurhash {
        return (data, thumbnail);
    }

    let content_type = content_type.clone();
    let info = config.info.take();

    let (data, thumbnail, info) = matrix_sdk_common::executor::spawn_blocking(move || {
        let (data, thumbnail) = if strip_exif {
            let data = exif::strip_metadata(&content_type, data);

            let thumbnail = thumbnail.map(|mut thumbnail| {
                thumbnail.data = exif::strip_metadata(&thumbnail.content_type, thumbnail.data);
                thumbnail.size = UInt::new_saturating(thumbnail.data.len() as u64);
                thumbnail
            });

            (data, thumbnail)
        } else {
            (data, thumbnail)
        };

        let info = if generate_blurhash {
            add_blurhash(&content_type, &data, thumbnail.as_ref(), info)
        } else {
            info
        };

        (data, thumbnail, info)
    })
    .await
    .expect("Preprocessing an attachment should never panic");

    config.info = info;

    (data, thumbnail)
}

/// Fill in the BlurHash of the attachment info, if it doesn't have one yet.
///
/// An image is hashed from its own data; a video from its thumbnail, since the
/// SDK has no video decoder. Anything else has nowhere to put a BlurHash.
#[cfg(feature = "image-proc")]
fn add_blurhash(
    content_type: &mime::Mime,
    data: &[u8],
    thumbnail: Option<&Thumbnail>,
    info: Option<AttachmentInfo>,
) -> Option<AttachmentInfo> {
    // With no info at all, only an image gets one made for it: for any other
    // type the SDK would be inventing metadata it knows nothing about.
    let info = info.or_else(|| {
        (content_type.type_() == mime::IMAGE)
            .then(|| AttachmentInfo::Image(BaseImageInfo::default()))
    })?;

    Some(match info {
        AttachmentInfo::Image(mut image) => {
            if image.blurhash.is_none() {
                image.blurhash = blurhash::compute(content_type, data);
            }

            AttachmentInfo::Image(image)
        }

        AttachmentInfo::Video(mut video) => {
            if video.blurhash.is_none()
                && let Some(thumbnail) = thumbnail
            {
                video.blurhash = blurhash::compute(&thumbnail.content_type, &thumbnail.data);
            }

            AttachmentInfo::Video(video)
        }

        info => info,
    })
}

/// Without the `image-proc` feature there is no image decoder to compute a
/// BlurHash with, so say so once rather than silently doing nothing.
#[cfg(not(feature = "image-proc"))]
fn add_blurhash(
    _content_type: &mime::Mime,
    _data: &[u8],
    _thumbnail: Option<&Thumbnail>,
    info: Option<AttachmentInfo>,
) -> Option<AttachmentInfo> {
    tracing::warn!(
        "a blurhash was requested for an attachment, but the SDK was built \
         without the `image-proc` feature"
    );

    info
}

/// Configuration for sending a gallery.
#[cfg(feature = "unstable-msc4274")]
#[derive(Debug, Default)]
pub struct GalleryConfig {
    pub(crate) txn_id: Option<OwnedTransactionId>,
    pub(crate) items: Vec<GalleryItemInfo>,
    pub(crate) caption: Option<TextMessageEventContent>,
    pub(crate) mentions: Option<Mentions>,
    pub(crate) reply: Option<Reply>,
}

#[cfg(feature = "unstable-msc4274")]
impl GalleryConfig {
    /// Create a new empty `GalleryConfig`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the transaction ID to send.
    ///
    /// # Arguments
    ///
    /// * `txn_id` - A unique ID that can be attached to a `MessageEvent` held
    ///   in its unsigned field as `transaction_id`. If not given, one is
    ///   created for the message.
    #[must_use]
    pub fn txn_id(mut self, txn_id: OwnedTransactionId) -> Self {
        self.txn_id = Some(txn_id);
        self
    }

    /// Adds a media item to the gallery.
    ///
    /// # Arguments
    ///
    /// * `item` - Information about the item to be added.
    #[must_use]
    pub fn add_item(mut self, item: GalleryItemInfo) -> Self {
        self.items.push(item);
        self
    }

    /// Set the optional caption.
    ///
    /// # Arguments
    ///
    /// * `caption` - The optional caption.
    pub fn caption(mut self, caption: Option<TextMessageEventContent>) -> Self {
        self.caption = caption;
        self
    }

    /// Set the mentions of the message.
    ///
    /// # Arguments
    ///
    /// * `mentions` - The mentions of the message.
    pub fn mentions(mut self, mentions: Option<Mentions>) -> Self {
        self.mentions = mentions;
        self
    }

    /// Set the reply information of the message.
    ///
    /// # Arguments
    ///
    /// * `reply` - The reply information of the message.
    pub fn reply(mut self, reply: Option<Reply>) -> Self {
        self.reply = reply;
        self
    }

    /// Returns the number of media items in the gallery.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Checks whether the gallery contains any media items or not.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(feature = "unstable-msc4274")]
#[derive(Debug)]
/// Metadata for a gallery item
pub struct GalleryItemInfo {
    /// The filename.
    pub filename: String,
    /// The mime type.
    pub content_type: mime::Mime,
    /// The binary data.
    pub data: Vec<u8>,
    /// The attachment info.
    pub attachment_info: AttachmentInfo,
    /// The caption.
    pub caption: Option<TextMessageEventContent>,
    /// The thumbnail.
    pub thumbnail: Option<Thumbnail>,
}
