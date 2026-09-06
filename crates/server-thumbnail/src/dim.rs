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
// Ported from the `Dim` type in tuwunel `src/service/media/thumbnail.rs`.

//! A requested thumbnail size.

use std::num::Saturating as Sat;

use common_ruma::{UInt, media::Method};

use crate::Error;

/// Dimension specification for a thumbnail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dim {
    /// The requested width in pixels.
    pub width: u32,

    /// The requested height in pixels.
    pub height: u32,

    /// Whether the picture is scaled to fit the request or cropped to fill it.
    pub method: Method,
}

impl Dim {
    /// Instantiates a `Dim` from Ruma integers with an optional method.
    ///
    /// # Errors
    ///
    /// Returns an error if either dimension does not fit in a `u32`.
    pub fn from_ruma(width: UInt, height: UInt, method: Option<Method>) -> Result<Self, Error> {
        let width = width.try_into().map_err(|_| Error::InvalidDimensions)?;
        let height = height.try_into().map_err(|_| Error::InvalidDimensions)?;

        Ok(Self::new(width, height, method))
    }

    /// Instantiates a `Dim` with an optional method, defaulting to scaling.
    #[inline]
    #[must_use]
    pub fn new(width: u32, height: u32, method: Option<Method>) -> Self {
        Self { width, height, method: method.unwrap_or(Method::Scale) }
    }

    /// The size this request scales to against a given source.
    ///
    /// The result keeps the source's aspect ratio, fits inside the request, and
    /// never exceeds the source in either dimension.
    ///
    /// This differs from tuwunel, whose `scaled` selects the looser of the two
    /// constraints and so returns a size that *covers* the request, exceeding
    /// it on one axis. The specification's `scale` method scales down to fit
    /// within the requested dimensions, which is what this returns.
    ///
    /// # Errors
    ///
    /// Returns an error if the source has a zero dimension.
    pub fn scaled(&self, source: &Self) -> Result<Self, Error> {
        let source_width = source.width;
        let source_height = source.height;

        let width = self.width.min(source_width);
        let height = self.height.min(source_height);

        // Whichever axis is the tighter constraint sets the scale, and the
        // other follows from the source's ratio. Comparing the two ratios as
        // cross products keeps this in integers.
        let width_is_tighter = Sat(width) * Sat(source_height) < Sat(height) * Sat(source_width);

        let (x, y) = if width_is_tighter {
            let dividend = (Sat(width) * Sat(source_height)).0;
            (width, dividend.checked_div(source_width).ok_or(Error::InvalidDimensions)?)
        } else {
            let dividend = (Sat(height) * Sat(source_width)).0;
            (dividend.checked_div(source_height).ok_or(Error::InvalidDimensions)?, height)
        };

        Ok(Self { width: x, height: y, method: Method::Scale })
    }

    /// Whether generation cannot improve on the source.
    ///
    /// True when the request would upscale, or when the generated thumbnail
    /// would carry the source's own dimensions; in both cases the original
    /// should be served instead of a generated copy of it.
    ///
    /// # Errors
    ///
    /// Returns an error if the source has a zero dimension.
    pub fn is_passthrough(&self, source: &Self) -> Result<bool, Error> {
        if self.width > source.width || self.height > source.height {
            return Ok(true);
        }

        let (width, height) = if self.crop() {
            (self.width, self.height)
        } else {
            let scaled = self.scaled(source)?;
            (scaled.width, scaled.height)
        };

        Ok(width == source.width && height == source.height)
    }

    /// The size a request rounds up to.
    ///
    /// The specification lists the sizes a server is expected to be able to
    /// serve; rounding a request up to one of them bounds how many distinct
    /// thumbnails a picture can produce. The requested method is ignored, since
    /// the listed sizes carry their own.
    #[must_use]
    pub fn normalized(&self) -> Self {
        match (self.width, self.height) {
            (0..=32, 0..=32) => Self::new(32, 32, Some(Method::Crop)),
            (0..=96, 0..=96) => Self::new(96, 96, Some(Method::Crop)),
            (0..=320, 0..=240) => Self::new(320, 240, Some(Method::Scale)),
            (0..=640, 0..=480) => Self::new(640, 480, Some(Method::Scale)),
            (0..=800, 0..=600) => Self::new(800, 600, Some(Method::Scale)),
            _ => Self::default(),
        }
    }

    /// Whether the method is [`Method::Crop`].
    #[inline]
    #[must_use]
    pub fn crop(&self) -> bool {
        self.method == Method::Crop
    }
}

impl Default for Dim {
    #[inline]
    fn default() -> Self {
        Self { width: 0, height: 0, method: Method::Scale }
    }
}

#[cfg(test)]
mod tests {
    use common_ruma::media::Method;

    use super::Dim;

    #[test]
    fn test_scaling_keeps_the_source_aspect_ratio() {
        let source = Dim::new(800, 400, None);

        // A 2:1 source inside a 320x240 box is bound by its width.
        let scaled = Dim::new(320, 240, None).scaled(&source).unwrap();
        assert_eq!((scaled.width, scaled.height), (320, 160));

        // A 1:2 source inside the same box is bound by its height.
        let scaled = Dim::new(320, 240, None).scaled(&Dim::new(400, 800, None)).unwrap();
        assert_eq!((scaled.width, scaled.height), (120, 240));
    }

    #[test]
    fn test_scaling_never_exceeds_the_source() {
        let source = Dim::new(64, 64, None);
        let scaled = Dim::new(320, 240, None).scaled(&source).unwrap();

        assert_eq!((scaled.width, scaled.height), (64, 64));
    }

    #[test]
    fn test_a_zero_sized_source_is_an_error() {
        assert!(Dim::new(320, 240, None).scaled(&Dim::new(0, 0, None)).is_err());
    }

    #[test]
    fn test_a_request_is_passthrough_when_generating_would_not_help() {
        let source = Dim::new(64, 64, None);

        // Larger than the source: it would upscale.
        assert!(Dim::new(320, 240, Some(Method::Scale)).is_passthrough(&source).unwrap());
        // Exactly the source: the thumbnail would be a copy.
        assert!(Dim::new(64, 64, Some(Method::Crop)).is_passthrough(&source).unwrap());
        // Smaller than the source: worth generating.
        assert!(!Dim::new(32, 32, Some(Method::Crop)).is_passthrough(&source).unwrap());
    }

    #[test]
    fn test_requests_round_up_to_a_listed_size() {
        assert_eq!(Dim::new(20, 20, None).normalized(), Dim::new(32, 32, Some(Method::Crop)));
        assert_eq!(Dim::new(64, 64, None).normalized(), Dim::new(96, 96, Some(Method::Crop)));
        assert_eq!(Dim::new(300, 200, None).normalized(), Dim::new(320, 240, Some(Method::Scale)));
        assert_eq!(Dim::new(700, 500, None).normalized(), Dim::new(800, 600, Some(Method::Scale)));

        // Past every listed size, so nothing is claimed about it.
        assert_eq!(Dim::new(4000, 3000, None).normalized(), Dim::default());
    }

    #[test]
    fn test_the_method_defaults_to_scaling() {
        assert!(!Dim::new(32, 32, None).crop());
        assert!(Dim::new(32, 32, Some(Method::Crop)).crop());
    }
}
