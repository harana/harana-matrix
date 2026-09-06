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

//! Computing the [BlurHash] of an image being uploaded.
//!
//! A BlurHash is a short string describing the rough colours of an image. A
//! receiving client can render it instantly, as a blurred placeholder, while
//! the actual media is still downloading. Nothing about it is secret: it is
//! sent in the clear inside the event content, even for an encrypted room, and
//! at this resolution it says no more than a heavily blurred thumbnail would.
//!
//! [BlurHash]: https://blurha.sh/

use image::{ImageFormat, imageops::FilterType};
use tracing::debug;

/// How many horizontal and vertical components the hash is made of.
///
/// Four by three is what other Matrix clients emit, and it keeps the hash short
/// enough to sit comfortably in an event.
const COMPONENTS_X: u32 = 4;
const COMPONENTS_Y: u32 = 3;

/// The longest side, in pixels, the image is scaled down to before hashing.
///
/// The hash only describes a handful of low-frequency components, so it makes
/// no difference to the result whether the source is 100 or 4000 pixels wide.
/// It makes a large difference to how long it takes.
const MAX_DIMENSION: u32 = 128;

/// Compute the BlurHash of an encoded image.
///
/// Returns `None` if the data cannot be decoded as an image of a format this
/// SDK understands, or if it has no pixels at all: a media event without a
/// BlurHash is perfectly valid, so a failure here is never worth failing the
/// upload over.
pub(crate) fn compute(content_type: &mime::Mime, data: &[u8]) -> Option<String> {
    let format = match content_type.subtype().as_str() {
        "jpeg" => ImageFormat::Jpeg,
        "png" => ImageFormat::Png,
        "webp" => ImageFormat::WebP,
        subtype => {
            debug!("not computing a blurhash: unsupported image format image/{subtype}");
            return None;
        }
    };

    let image = match image::load_from_memory_with_format(data, format) {
        Ok(image) => image,
        Err(error) => {
            debug!("not computing a blurhash: the image could not be decoded: {error}");
            return None;
        }
    };

    // Scaling down first is what makes this cheap enough to run on every
    // upload. `Triangle` is a reasonable trade: cheaper than Lanczos, and the
    // result is about to be reduced to a dozen coefficients anyway.
    let image = image.resize(MAX_DIMENSION, MAX_DIMENSION, FilterType::Triangle).into_rgba8();
    let (width, height) = image.dimensions();

    if width == 0 || height == 0 {
        return None;
    }

    match blurhash::encode(COMPONENTS_X, COMPONENTS_Y, width, height, image.as_raw()) {
        Ok(blurhash) => Some(blurhash),
        Err(error) => {
            debug!("not computing a blurhash: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{ImageFormat, Rgba, RgbaImage};

    use super::{COMPONENTS_X, COMPONENTS_Y, compute};

    /// Encode a solid-colour image, to have something real to decode.
    fn solid_png(width: u32, height: u32, colour: [u8; 4]) -> Vec<u8> {
        let image = RgbaImage::from_pixel(width, height, Rgba(colour));

        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn test_compute_blurhash() {
        let red = compute(&mime::IMAGE_PNG, &solid_png(64, 48, [255, 0, 0, 255]))
            .expect("a solid red PNG should hash");
        let blue = compute(&mime::IMAGE_PNG, &solid_png(64, 48, [0, 0, 255, 255]))
            .expect("a solid blue PNG should hash");

        // A 4x3 hash starts with a single character encoding the component
        // counts, and is a fixed length from there.
        assert_eq!(red.len(), 6 + 2 * (COMPONENTS_X * COMPONENTS_Y - 1) as usize);

        // Different images hash differently, and the same image hashes the
        // same way twice.
        assert_ne!(red, blue);
        assert_eq!(red, compute(&mime::IMAGE_PNG, &solid_png(64, 48, [255, 0, 0, 255])).unwrap());

        // The hash describes the colours, not the size, so scaling the same
        // solid colour changes nothing.
        assert_eq!(red, compute(&mime::IMAGE_PNG, &solid_png(16, 12, [255, 0, 0, 255])).unwrap());
    }

    #[test]
    fn test_compute_blurhash_of_a_large_image() {
        // Larger than `MAX_DIMENSION`, so it goes through the downscaling
        // path.
        let hash = compute(&mime::IMAGE_PNG, &solid_png(500, 400, [0, 128, 0, 255]));
        assert!(hash.is_some());
    }

    #[test]
    fn test_undecodable_data_has_no_blurhash() {
        // A format we don't decode.
        assert_eq!(compute(&"image/gif".parse().unwrap(), b"GIF89a"), None);

        // A content type we do decode, over data that isn't an image.
        assert_eq!(compute(&mime::IMAGE_PNG, b"not a png"), None);

        // Nothing at all.
        assert_eq!(compute(&mime::IMAGE_JPEG, b""), None);
    }
}
