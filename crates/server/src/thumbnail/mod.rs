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
// Ported from tuwunel `src/service/media/thumbnail.rs`.

#![doc = include_str!("../../docs/thumbnail.md")]
#![warn(missing_docs, missing_debug_implementations)]

mod dim;

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, ImageReader, Limits, imageops::FilterType};
use tracing::{debug, instrument};

pub use self::dim::Dim;

/// Content type of every thumbnail this crate generates.
pub const PNG: &str = "image/png";

/// Filename a generated thumbnail is disposed under.
///
/// The media repository specification names the generated thumbnail rather than
/// the file it was generated from.
pub const THUMBNAIL_NAME: &str = "thumbnail.png";

/// Bytes the decoder is budgeted per pixel of the picture it is asked for.
const BYTES_PER_PIXEL: u64 = 4;

/// A reasonable default pixel budget for a decoded picture.
///
/// Large enough for a 50 megapixel photograph, which bounds the decoder's
/// allocation at roughly 200 MB.
pub const DEFAULT_MAX_PIXELS: u64 = 50_000_000;

/// Thumbnail generation that could not be completed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The picture's dimensions could not be read, or it could not be decoded.
    #[error(transparent)]
    Image(#[from] image::ImageError),

    /// The picture's format could not be guessed from its bytes.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The picture declares more pixels than the caller's budget allows.
    ///
    /// This is checked against the header before decoding, so a small file
    /// declaring an enormous canvas is rejected without allocating for it.
    #[error("picture of {width}x{height} is past the {budget} pixel budget")]
    PastPixelBudget {
        /// The width the picture's header declares.
        width: u32,
        /// The height the picture's header declares.
        height: u32,
        /// The budget it exceeded.
        budget: u64,
    },

    /// The requested dimensions overflow while being scaled.
    #[error("the requested dimensions cannot be scaled")]
    InvalidDimensions,
}

/// Decodes, scales and encodes a thumbnail in one call.
///
/// The result is a PNG, whatever the source format. `max_pixels` bounds what
/// the decoder is asked to allocate; [`DEFAULT_MAX_PIXELS`] is a reasonable
/// value when the caller has no policy of its own.
///
/// # Errors
///
/// Returns an error if the picture cannot be read, exceeds `max_pixels`, or
/// cannot be encoded.
#[instrument(level = "debug", skip(bytes), fields(len = bytes.len()))]
pub fn thumbnail(bytes: &[u8], requested: &Dim, max_pixels: u64) -> Result<Vec<u8>, Error> {
    let image = decode(bytes, max_pixels)?;
    let thumbnail = generate(&image, requested)?;

    encode(&thumbnail)
}

/// Decodes a picture whose header declares no more than `max_pixels`.
///
/// The dimensions are checked before any decoder allocates, since the
/// decoder's [`Limits`] enforce only a byte budget and leave a decoder free to
/// ignore it.
///
/// # Errors
///
/// Returns an error if the picture's format or dimensions cannot be read, if it
/// declares more than `max_pixels`, or if decoding fails.
#[instrument(level = "trace", skip_all)]
pub fn decode(bytes: &[u8], max_pixels: u64) -> Result<DynamicImage, Error> {
    let (width, height) = reader(bytes)?.into_dimensions()?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));

    if pixels > max_pixels {
        debug!(%width, %height, "picture is past the {max_pixels} pixel budget");
        return Err(Error::PastPixelBudget { width, height, budget: max_pixels });
    }

    let mut limits = Limits::no_limits();
    limits.max_alloc = Some(max_pixels.saturating_mul(BYTES_PER_PIXEL));

    let mut reader = reader(bytes)?;
    reader.limits(limits);

    Ok(reader.decode()?)
}

/// Scales or crops a decoded picture to the requested dimensions.
///
/// Upscaling is forbidden outright: a request larger than the source yields the
/// source's own dimensions rather than an enlargement.
///
/// # Errors
///
/// Returns an error if the requested dimensions cannot be scaled against the
/// source's.
pub fn generate(image: &DynamicImage, requested: &Dim) -> Result<DynamicImage, Error> {
    let source = Dim::new(image.width(), image.height(), None);

    let thumbnail = if requested.crop() {
        // `resize_to_fill` enlarges a source smaller than the request to meet
        // it, so the request is clamped to the source first.
        let width = requested.width.min(image.width());
        let height = requested.height.min(image.height());

        image.resize_to_fill(width, height, FilterType::CatmullRom)
    } else {
        let Dim { width, height, .. } = requested.scaled(&source)?;

        image.thumbnail_exact(width, height)
    };

    Ok(thumbnail)
}

/// Encodes a picture as PNG.
///
/// # Errors
///
/// Returns an error if encoding fails.
pub fn encode(image: &DynamicImage) -> Result<Vec<u8>, Error> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png)?;

    Ok(bytes.into_inner())
}

fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>, Error> {
    Ok(ImageReader::new(Cursor::new(bytes)).with_guessed_format()?)
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use harana_matrix_common::media::Method;

    use super::{DEFAULT_MAX_PIXELS, Dim, Error, decode, generate, thumbnail};

    fn picture(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(width, height));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();

        bytes.into_inner()
    }

    #[test]
    fn test_a_scaled_thumbnail_keeps_the_aspect_ratio() {
        let image = decode(&picture(800, 400), DEFAULT_MAX_PIXELS).unwrap();
        let thumbnail = generate(&image, &Dim::new(320, 240, Some(Method::Scale))).unwrap();

        // 320x240 against a 2:1 source scales to 320x160.
        assert_eq!((thumbnail.width(), thumbnail.height()), (320, 160));
    }

    #[test]
    fn test_a_cropped_thumbnail_fills_the_requested_box() {
        let image = decode(&picture(800, 400), DEFAULT_MAX_PIXELS).unwrap();
        let thumbnail = generate(&image, &Dim::new(96, 96, Some(Method::Crop))).unwrap();

        assert_eq!((thumbnail.width(), thumbnail.height()), (96, 96));
    }

    #[test]
    fn test_a_request_larger_than_the_source_does_not_upscale() {
        let image = decode(&picture(64, 64), DEFAULT_MAX_PIXELS).unwrap();

        let scaled = generate(&image, &Dim::new(320, 240, Some(Method::Scale))).unwrap();
        assert_eq!((scaled.width(), scaled.height()), (64, 64));

        let cropped = generate(&image, &Dim::new(320, 240, Some(Method::Crop))).unwrap();
        assert_eq!((cropped.width(), cropped.height()), (64, 64));
    }

    #[test]
    fn test_a_picture_past_the_budget_is_refused_before_decoding() {
        // The budget is one pixel short of the picture, so it is refused on its
        // header rather than after allocating for it.
        let error = decode(&picture(64, 64), (64 * 64) - 1).unwrap_err();

        assert!(
            matches!(error, Error::PastPixelBudget { width: 64, height: 64, .. }),
            "{error:?} should report the budget"
        );
    }

    #[test]
    fn test_a_generated_thumbnail_is_a_png() {
        let jpeg = {
            let image = DynamicImage::ImageRgb8(image::RgbImage::new(200, 100));
            let mut bytes = std::io::Cursor::new(Vec::new());
            image.write_to(&mut bytes, ImageFormat::Jpeg).unwrap();
            bytes.into_inner()
        };

        let thumbnail =
            thumbnail(&jpeg, &Dim::new(96, 96, Some(Method::Crop)), DEFAULT_MAX_PIXELS).unwrap();

        assert_eq!(image::guess_format(&thumbnail).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn test_a_picture_that_is_not_a_picture_is_an_error() {
        assert!(decode(b"not a picture at all", DEFAULT_MAX_PIXELS).is_err());
    }
}
