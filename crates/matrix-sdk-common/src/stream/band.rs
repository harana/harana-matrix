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
// Ported from tuwunel `src/core/utils/stream/band.rs`.

//! Live concurrency settings shared by the stream combinators.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Stream concurrency factor; this is a live value.
static WIDTH: AtomicUsize = AtomicUsize::new(32);

/// Stream throughput amplifier; this is a live value.
static AMPLIFICATION: AtomicUsize = AtomicUsize::new(1024);

/// Practicable limits on the stream width.
pub const WIDTH_LIMIT: (usize, usize) = (1, 1024);

/// Practicable limits on the stream amplifier.
pub const AMPLIFICATION_LIMIT: (usize, usize) = (32, 32768);

/// Sets the live concurrency factor.
///
/// The first return value is the previous width which was replaced. The second
/// return value is the value which was set after any applied limits.
pub fn set_width(width: usize) -> (usize, usize) {
    let width = width.clamp(WIDTH_LIMIT.0, WIDTH_LIMIT.1);
    (WIDTH.swap(width, Ordering::Relaxed), width)
}

/// Sets the live concurrency amplification.
///
/// The first return value is the previous amplification which was replaced. The
/// second return value is the value which was set after any applied limits.
pub fn set_amplification(amplification: usize) -> (usize, usize) {
    let amplification = amplification.clamp(AMPLIFICATION_LIMIT.0, AMPLIFICATION_LIMIT.1);
    (AMPLIFICATION.swap(amplification, Ordering::Relaxed), amplification)
}

/// Returns the live default concurrency width for stream operations.
///
/// Operations with an explicit width bypass this default; updates take effect
/// immediately for every later operation which does not.
#[inline]
pub fn automatic_width() -> usize {
    let width = WIDTH.load(Ordering::Relaxed);
    debug_assert!(width >= WIDTH_LIMIT.0, "WIDTH should not be zero");
    debug_assert!(width <= WIDTH_LIMIT.1, "WIDTH is probably too large");
    width
}

/// Returns the live amplification for operations which batch their work.
///
/// Used by stream operations where the amplification hasn't been manually
/// supplied by the caller.
#[inline]
pub fn automatic_amplification() -> usize {
    let amplification = AMPLIFICATION.load(Ordering::Relaxed);
    debug_assert!(amplification >= AMPLIFICATION_LIMIT.0, "amplification is too low");
    debug_assert!(amplification <= AMPLIFICATION_LIMIT.1, "amplification is too high");
    amplification
}

#[cfg(test)]
mod tests {
    use super::{
        AMPLIFICATION_LIMIT, WIDTH_LIMIT, automatic_amplification, automatic_width,
        set_amplification, set_width,
    };

    // The width and amplification are process-wide, so a single test owns them
    // rather than racing sibling tests in the same binary.
    #[test]
    fn test_settings_are_clamped_and_return_the_previous_value() {
        let (_previous, set) = set_width(WIDTH_LIMIT.1 * 2);
        assert_eq!(set, WIDTH_LIMIT.1);
        assert_eq!(automatic_width(), WIDTH_LIMIT.1);

        let (previous, set) = set_width(0);
        assert_eq!(previous, WIDTH_LIMIT.1);
        assert_eq!(set, WIDTH_LIMIT.0);
        assert_eq!(automatic_width(), WIDTH_LIMIT.0);

        let (_previous, set) = set_amplification(0);
        assert_eq!(set, AMPLIFICATION_LIMIT.0);
        assert_eq!(automatic_amplification(), AMPLIFICATION_LIMIT.0);

        set_width(32);
        set_amplification(1024);
    }
}
