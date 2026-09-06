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

//! Removing the metadata a camera embeds in an image.
//!
//! A photo straight off a phone carries an Exif block with, among other things,
//! the GPS coordinates and the wall-clock time of the shot, the device model
//! and sometimes its serial number. Uploading the file as-is hands all of that
//! to everyone in the room, and to the homeserver's media repository.
//!
//! This module rewrites the container so that none of it survives, with one
//! deliberate exception: the Exif `Orientation` tag, which says how the decoder
//! should rotate the pixels. Dropping it would show a lot of photos sideways,
//! and it says nothing about where or when the photo was taken.
//!
//! The pixels themselves are never touched: this walks the container's segment
//! or chunk structure and copies everything but the metadata blocks, so there
//! is no re-encoding and no quality loss.
//!
//! # Supported formats
//!
//! JPEG, PNG and WebP. Any other content type is returned unchanged, including
//! HEIC/HEIF, which phones also use for photos: its metadata lives in an
//! ISO-BMFF box structure this module does not parse yet.

use tracing::debug;

/// The Exif tag holding the orientation of the image, from the TIFF 6.0
/// specification.
const TAG_ORIENTATION: u16 = 0x0112;

/// The Exif value type for a 16-bit unsigned integer.
const TYPE_SHORT: u16 = 3;

/// The size of an entry in a TIFF image file directory.
const IFD_ENTRY_LEN: usize = 12;

/// The `Exif\0\0` identifier that introduces the TIFF block of a JPEG `APP1`
/// segment.
const JPEG_EXIF_ID: &[u8] = b"Exif\0\0";

/// Strip the metadata of an image, keeping its orientation.
///
/// `content_type` is the MIME type the attachment will be uploaded with. If it
/// does not name a format this module understands, or if the data does not
/// actually look like that format, the data is returned unchanged: a failure to
/// strip metadata must not turn into a failure to send the image.
pub(crate) fn strip_metadata(content_type: &mime::Mime, data: Vec<u8>) -> Vec<u8> {
    if content_type.type_() != mime::IMAGE {
        return data;
    }

    let stripped = match content_type.subtype().as_str() {
        "jpeg" => strip_jpeg(&data),
        "png" => strip_png(&data),
        "webp" => strip_webp(&data),
        subtype => {
            debug!("not stripping metadata: unsupported image format image/{subtype}");
            None
        }
    };

    match stripped {
        Some(stripped) => stripped,
        None => {
            debug!("not stripping metadata: the image could not be parsed");
            data
        }
    }
}

/// Read the value of the orientation tag out of a TIFF block, the structure
/// that Exif data is made of in every container.
///
/// Returns `None` if the block is malformed or has no orientation, which is
/// the same thing as far as callers are concerned: there is nothing to carry
/// over.
fn read_orientation(tiff: &[u8]) -> Option<u16> {
    // The TIFF header: two bytes of byte order, the magic number 42, and the
    // offset of the first image file directory.
    let big_endian = match tiff.get(..2)? {
        b"II" => false,
        b"MM" => true,
        _ => return None,
    };

    let u16_at = |offset: usize| -> Option<u16> {
        let bytes = tiff.get(offset..offset + 2)?.try_into().ok()?;
        Some(if big_endian { u16::from_be_bytes(bytes) } else { u16::from_le_bytes(bytes) })
    };
    let u32_at = |offset: usize| -> Option<u32> {
        let bytes = tiff.get(offset..offset + 4)?.try_into().ok()?;
        Some(if big_endian { u32::from_be_bytes(bytes) } else { u32::from_le_bytes(bytes) })
    };

    if u16_at(2)? != 42 {
        return None;
    }

    let ifd_offset = u32_at(4)? as usize;
    let entry_count = u16_at(ifd_offset)? as usize;

    for index in 0..entry_count {
        let entry = ifd_offset + 2 + index * IFD_ENTRY_LEN;

        if u16_at(entry)? != TAG_ORIENTATION {
            continue;
        }

        // A well-formed orientation is a single `SHORT`, stored inline in the
        // entry's value field because it fits in four bytes.
        if u16_at(entry + 2)? != TYPE_SHORT || u32_at(entry + 4)? != 1 {
            return None;
        }

        // Only the eight values defined by TIFF 6.0 mean anything; anything
        // else is as good as absent.
        return u16_at(entry + 8).filter(|orientation| (1..=8).contains(orientation));
    }

    None
}

/// Build a TIFF block holding nothing but the given orientation.
///
/// This is what replaces the original Exif block: a header, one image file
/// directory with a single entry, and no next directory.
fn minimal_tiff(orientation: u16) -> Vec<u8> {
    let mut tiff = Vec::with_capacity(26);

    // Header: little endian, magic 42, first directory at offset 8.
    tiff.extend_from_slice(b"II");
    tiff.extend_from_slice(&42u16.to_le_bytes());
    tiff.extend_from_slice(&8u32.to_le_bytes());

    // One entry, whose value fits in the entry itself.
    tiff.extend_from_slice(&1u16.to_le_bytes());
    tiff.extend_from_slice(&TAG_ORIENTATION.to_le_bytes());
    tiff.extend_from_slice(&TYPE_SHORT.to_le_bytes());
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&orientation.to_le_bytes());
    tiff.extend_from_slice(&[0, 0]);

    // No next directory.
    tiff.extend_from_slice(&0u32.to_le_bytes());

    tiff
}

/// What to do with a JPEG segment.
enum JpegSegment {
    /// Copy it over as-is.
    Keep,
    /// Drop it: it holds metadata.
    Drop,
    /// It is an Exif block; rewrite it down to the orientation.
    Exif,
}

/// Decide the fate of a JPEG segment from its marker and payload.
///
/// Everything a decoder needs is kept: the JFIF header, the ICC colour
/// profile, the Adobe colour transform marker, and of course the frame,
/// quantisation and Huffman tables. The application segments that exist to
/// carry metadata go away, and so do comments, which are free-form enough to
/// hold anything.
fn classify_jpeg_segment(marker: u8, payload: &[u8]) -> JpegSegment {
    match marker {
        // APP1 holds either Exif or XMP; the latter is metadata through and
        // through, and so is anything else that lands here.
        0xE1 => {
            if payload.starts_with(JPEG_EXIF_ID) {
                JpegSegment::Exif
            } else {
                JpegSegment::Drop
            }
        }
        // APP0 (JFIF), APP2 (ICC profile), APP14 (Adobe): a decoder wants
        // these.
        0xE0 | 0xE2 | 0xEE => JpegSegment::Keep,
        // APP3 to APP13: maker notes, IPTC records, editing history. None of
        // it is needed to display the image.
        0xE3..=0xED => JpegSegment::Drop,
        // COM: a free-form comment.
        0xFE => JpegSegment::Drop,
        _ => JpegSegment::Keep,
    }
}

/// Strip a JPEG file, keeping its orientation.
///
/// Returns `None` if the data is not a JPEG we can walk, in which case the
/// caller leaves it alone.
fn strip_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    if data.get(..2)? != [0xFF, 0xD8] {
        return None;
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..2]);

    let mut pos = 2;

    loop {
        // Markers are introduced by one or more `0xFF` fill bytes.
        if *data.get(pos)? != 0xFF {
            return None;
        }
        while *data.get(pos)? == 0xFF {
            pos += 1;
        }

        let marker = *data.get(pos)?;
        pos += 1;

        match marker {
            // Standalone markers: no payload follows.
            0x01 | 0xD0..=0xD7 => {
                out.extend_from_slice(&[0xFF, marker]);
                continue;
            }
            // Start of scan: the entropy-coded data runs to the end of the
            // file, and holds no metadata. Copy the rest verbatim.
            0xDA => {
                out.extend_from_slice(&data[pos - 2..]);
                return Some(out);
            }
            _ => {}
        }

        let length = u16::from_be_bytes(data.get(pos..pos + 2)?.try_into().ok()?) as usize;
        // The length counts itself, so a segment shorter than that is
        // nonsense.
        let payload = data.get(pos + 2..pos + length.checked_sub(2)? + 2)?;

        match classify_jpeg_segment(marker, payload) {
            JpegSegment::Keep => {
                out.extend_from_slice(&[0xFF, marker]);
                out.extend_from_slice(&data[pos..pos + length]);
            }

            JpegSegment::Drop => {}

            JpegSegment::Exif => {
                if let Some(orientation) = read_orientation(&payload[JPEG_EXIF_ID.len()..]) {
                    let tiff = minimal_tiff(orientation);
                    let segment_length = (2 + JPEG_EXIF_ID.len() + tiff.len()) as u16;

                    out.extend_from_slice(&[0xFF, marker]);
                    out.extend_from_slice(&segment_length.to_be_bytes());
                    out.extend_from_slice(JPEG_EXIF_ID);
                    out.extend_from_slice(&tiff);
                }
            }
        }

        pos += length;
    }
}

/// The CRC-32 that PNG uses to protect each chunk, computed the slow but
/// obvious way. The only chunk this module ever builds is a few dozen bytes
/// long, so a table would not pay for itself.
fn png_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;

    for byte in bytes {
        crc ^= u32::from(*byte);

        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }

    !crc
}

/// Append a PNG chunk, with its length and CRC, to `out`.
fn push_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);

    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&png_crc32(&crc_input).to_be_bytes());
}

/// Strip a PNG file, keeping its orientation.
///
/// Returns `None` if the data is not a PNG we can walk, in which case the
/// caller leaves it alone.
fn strip_png(data: &[u8]) -> Option<Vec<u8>> {
    const SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    if data.get(..SIGNATURE.len())? != SIGNATURE {
        return None;
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(SIGNATURE);

    let mut pos = SIGNATURE.len();

    while pos < data.len() {
        let length = u32::from_be_bytes(data.get(pos..pos + 4)?.try_into().ok()?) as usize;
        let chunk_type: &[u8; 4] = data.get(pos + 4..pos + 8)?.try_into().ok()?;
        let chunk_data = data.get(pos + 8..pos + 8 + length)?;
        // Make sure the CRC is there before we commit to this chunk.
        let end = pos + 8 + length + 4;
        data.get(pos..end)?;

        match chunk_type {
            // The Exif block, in the same TIFF form as everywhere else.
            b"eXIf" => {
                if let Some(orientation) = read_orientation(chunk_data) {
                    push_png_chunk(&mut out, b"eXIf", &minimal_tiff(orientation));
                }
            }

            // Free-form text and a last-modification timestamp: both are
            // metadata, and text chunks are where PNG writers put whatever
            // they like.
            b"tEXt" | b"zTXt" | b"iTXt" | b"tIME" => {}

            _ => out.extend_from_slice(&data[pos..end]),
        }

        pos = end;
    }

    Some(out)
}

/// The `VP8X` flag saying the file carries an Exif chunk.
const WEBP_FLAG_EXIF: u8 = 0x08;

/// The `VP8X` flag saying the file carries an XMP chunk.
const WEBP_FLAG_XMP: u8 = 0x04;

/// Strip a WebP file, keeping its orientation.
///
/// Returns `None` if the data is not a WebP we can walk, in which case the
/// caller leaves it alone.
fn strip_webp(data: &[u8]) -> Option<Vec<u8>> {
    if data.get(..4)? != b"RIFF" || data.get(8..12)? != b"WEBP" {
        return None;
    }

    // The RIFF size counts everything after it, so it has to be rewritten once
    // we know how much is left. Start the body at the `WEBP` form type.
    let mut body = Vec::with_capacity(data.len());
    body.extend_from_slice(b"WEBP");

    let mut pos = 12;
    // Where the `VP8X` flags byte ends up in `body`, so it can be corrected
    // after the fact: the XMP chunk it advertises is about to disappear.
    let mut flags_offset = None;

    while pos < data.len() {
        let chunk_type: &[u8; 4] = data.get(pos..pos + 4)?.try_into().ok()?;
        let length = u32::from_le_bytes(data.get(pos + 4..pos + 8)?.try_into().ok()?) as usize;
        let chunk_data = data.get(pos + 8..pos + 8 + length)?;
        // Chunks are padded to an even size.
        let end = pos + 8 + length + (length % 2);

        match chunk_type {
            b"EXIF" => {
                if let Some(orientation) = read_orientation(chunk_data) {
                    let tiff = minimal_tiff(orientation);

                    body.extend_from_slice(b"EXIF");
                    body.extend_from_slice(&(tiff.len() as u32).to_le_bytes());
                    body.extend_from_slice(&tiff);

                    if !tiff.len().is_multiple_of(2) {
                        body.push(0);
                    }
                } else if let Some(flags_offset) = flags_offset {
                    body[flags_offset] &= !WEBP_FLAG_EXIF;
                }
            }

            b"XMP " => {
                if let Some(flags_offset) = flags_offset {
                    body[flags_offset] &= !WEBP_FLAG_XMP;
                }
            }

            _ => {
                if chunk_type == b"VP8X" {
                    // The flags are the first byte of the chunk's payload.
                    flags_offset = Some(body.len() + 8);
                }

                body.extend_from_slice(data.get(pos..end.min(data.len()))?);
            }
        }

        pos = end;
    }

    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{
        JPEG_EXIF_ID, TAG_ORIENTATION, TYPE_SHORT, minimal_tiff, push_png_chunk, read_orientation,
        strip_metadata,
    };

    /// A TIFF block with an orientation, a timestamp and a GPS reference, so
    /// there is something to lose.
    fn tiff_with_secrets(orientation: u16) -> Vec<u8> {
        let entries: [(u16, u16, u32, [u8; 4]); 3] = [
            (TAG_ORIENTATION, TYPE_SHORT, 1, [orientation as u8, 0, 0, 0]),
            // GPSInfo, pointing at an offset we never write; the parser must
            // not care, and the stripper must not keep it.
            (0x8825, 4, 1, [0xFF, 0xFF, 0, 0]),
            // DateTimeOriginal.
            (0x9003, 2, 4, *b"now\0"),
        ];

        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"II");
        tiff.extend_from_slice(&42u16.to_le_bytes());
        tiff.extend_from_slice(&8u32.to_le_bytes());
        tiff.extend_from_slice(&(entries.len() as u16).to_le_bytes());

        for (tag, ty, count, value) in entries {
            tiff.extend_from_slice(&tag.to_le_bytes());
            tiff.extend_from_slice(&ty.to_le_bytes());
            tiff.extend_from_slice(&count.to_le_bytes());
            tiff.extend_from_slice(&value);
        }

        tiff.extend_from_slice(&0u32.to_le_bytes());
        tiff
    }

    fn jpeg_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut segment = vec![0xFF, marker];
        segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        segment.extend_from_slice(payload);
        segment
    }

    /// A JPEG carrying an Exif block, an XMP block, a comment, an ICC profile
    /// and some scan data.
    fn jpeg_with_secrets(orientation: u16) -> Vec<u8> {
        let mut exif = JPEG_EXIF_ID.to_vec();
        exif.extend_from_slice(&tiff_with_secrets(orientation));

        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&jpeg_segment(0xE0, b"JFIF\0secretless"));
        jpeg.extend_from_slice(&jpeg_segment(0xE1, &exif));
        jpeg.extend_from_slice(&jpeg_segment(
            0xE1,
            b"http://ns.adobe.com/xap/1.0/\0<xmp>gps</xmp>",
        ));
        jpeg.extend_from_slice(&jpeg_segment(0xED, b"Photoshop 3.0\0IPTC city name"));
        jpeg.extend_from_slice(&jpeg_segment(0xFE, b"taken at home"));
        jpeg.extend_from_slice(&jpeg_segment(0xE2, b"ICC_PROFILE\0colours"));
        jpeg.extend_from_slice(&jpeg_segment(0xDB, b"quantisation"));
        // Start of scan, then entropy-coded data to the end of the file.
        jpeg.extend_from_slice(&jpeg_segment(0xDA, b"scan header"));
        jpeg.extend_from_slice(b"entropy coded data");
        jpeg.extend_from_slice(&[0xFF, 0xD9]);
        jpeg
    }

    fn png_with_secrets(orientation: u16) -> Vec<u8> {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        push_png_chunk(&mut png, b"IHDR", b"header bytes");
        push_png_chunk(&mut png, b"eXIf", &tiff_with_secrets(orientation));
        push_png_chunk(&mut png, b"tEXt", b"Comment\0taken at home");
        push_png_chunk(&mut png, b"iTXt", b"XML:com.adobe.xmp\0\0\0\0\0<xmp>gps</xmp>");
        push_png_chunk(&mut png, b"tIME", b"\x07\xe8\x01\x01\x00\x00\x00");
        push_png_chunk(&mut png, b"IDAT", b"pixels");
        push_png_chunk(&mut png, b"IEND", b"");
        png
    }

    fn webp_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(chunk_type);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);

        if !data.len().is_multiple_of(2) {
            out.push(0);
        }
    }

    fn webp_with_secrets(orientation: u16) -> Vec<u8> {
        let mut body = b"WEBP".to_vec();
        // A `VP8X` chunk announcing both Exif and XMP, with a canvas size.
        webp_chunk(&mut body, b"VP8X", &[0x0C, 0, 0, 0, 0x3F, 0, 0, 0x3F, 0, 0]);
        webp_chunk(&mut body, b"VP8 ", b"pixels");
        webp_chunk(&mut body, b"EXIF", &tiff_with_secrets(orientation));
        webp_chunk(&mut body, b"XMP ", b"<xmp>gps</xmp>");

        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&(body.len() as u32).to_le_bytes());
        webp.extend_from_slice(&body);
        webp
    }

    #[test]
    fn test_read_orientation() {
        assert_eq!(read_orientation(&tiff_with_secrets(6)), Some(6));
        assert_eq!(read_orientation(&minimal_tiff(3)), Some(3));

        // An orientation outside the values TIFF defines is not one.
        assert_eq!(read_orientation(&tiff_with_secrets(42)), None);

        // Garbage in, nothing out.
        assert_eq!(read_orientation(b""), None);
        assert_eq!(read_orientation(b"II"), None);
        assert_eq!(read_orientation(b"XX\x2a\x00\x08\x00\x00\x00\x00\x00"), None);
    }

    #[test]
    fn test_big_endian_tiff() {
        let mut tiff = Vec::new();
        tiff.extend_from_slice(b"MM");
        tiff.extend_from_slice(&42u16.to_be_bytes());
        tiff.extend_from_slice(&8u32.to_be_bytes());
        tiff.extend_from_slice(&1u16.to_be_bytes());
        tiff.extend_from_slice(&TAG_ORIENTATION.to_be_bytes());
        tiff.extend_from_slice(&TYPE_SHORT.to_be_bytes());
        tiff.extend_from_slice(&1u32.to_be_bytes());
        tiff.extend_from_slice(&8u16.to_be_bytes());
        tiff.extend_from_slice(&[0, 0]);
        tiff.extend_from_slice(&0u32.to_be_bytes());

        assert_eq!(read_orientation(&tiff), Some(8));
    }

    #[test]
    fn test_strip_jpeg() {
        let stripped = strip_metadata(&mime::IMAGE_JPEG, jpeg_with_secrets(6));

        // The Exif block is down to its orientation.
        assert_eq!(read_orientation_of_jpeg(&stripped), Some(6));

        // And nothing else survived.
        assert!(!contains(&stripped, b"gps"));
        assert!(!contains(&stripped, b"IPTC city name"));
        assert!(!contains(&stripped, b"taken at home"));
        assert!(!contains(&stripped, b"now"));

        // While the image itself did.
        assert!(contains(&stripped, b"JFIF\0secretless"));
        assert!(contains(&stripped, b"ICC_PROFILE\0colours"));
        assert!(contains(&stripped, b"quantisation"));
        assert!(contains(&stripped, b"scan header"));
        assert!(contains(&stripped, b"entropy coded data"));
        assert_eq!(&stripped[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_strip_jpeg_without_orientation() {
        let mut exif = JPEG_EXIF_ID.to_vec();
        exif.extend_from_slice(&tiff_with_secrets(42));

        let mut jpeg = vec![0xFF, 0xD8];
        jpeg.extend_from_slice(&jpeg_segment(0xE1, &exif));
        jpeg.extend_from_slice(&jpeg_segment(0xDA, b"scan header"));
        jpeg.extend_from_slice(b"entropy coded data");

        let stripped = strip_metadata(&mime::IMAGE_JPEG, jpeg);

        // With no orientation to carry over, the Exif segment goes entirely.
        assert!(!contains(&stripped, JPEG_EXIF_ID));
        assert!(contains(&stripped, b"entropy coded data"));
    }

    #[test]
    fn test_strip_png() {
        let stripped = strip_metadata(&mime::IMAGE_PNG, png_with_secrets(3));

        assert!(!contains(&stripped, b"taken at home"));
        assert!(!contains(&stripped, b"<xmp>gps</xmp>"));
        assert!(!contains(&stripped, b"tIME"));

        assert!(contains(&stripped, b"header bytes"));
        assert!(contains(&stripped, b"pixels"));
        assert!(contains(&stripped, b"IEND"));

        // The rebuilt `eXIf` chunk is a well-formed chunk carrying the
        // orientation and nothing else.
        let exif = png_chunk(&stripped, b"eXIf").expect("the eXIf chunk should still be there");
        assert_eq!(read_orientation(exif), Some(3));
        assert_eq!(exif, minimal_tiff(3));
    }

    #[test]
    fn test_strip_webp() {
        let stripped = strip_metadata(&"image/webp".parse().unwrap(), webp_with_secrets(8));

        assert!(!contains(&stripped, b"<xmp>gps</xmp>"));
        assert!(contains(&stripped, b"pixels"));

        // The RIFF size still describes the file.
        let size = u32::from_le_bytes(stripped[4..8].try_into().unwrap()) as usize;
        assert_eq!(size, stripped.len() - 8);

        // The XMP flag is gone from `VP8X`, the Exif flag stays.
        let flags = stripped[20];
        assert_eq!(flags & 0x04, 0);
        assert_eq!(flags & 0x08, 0x08);

        let exif = webp_chunk_data(&stripped, b"EXIF").expect("the EXIF chunk should still exist");
        assert_eq!(read_orientation(exif), Some(8));
    }

    #[test]
    fn test_unsupported_data_is_left_alone() {
        // A format we don't parse.
        let gif = b"GIF89a with a comment".to_vec();
        assert_eq!(strip_metadata(&"image/gif".parse().unwrap(), gif.clone()), gif);

        // Something that isn't an image at all.
        let text = b"hello".to_vec();
        assert_eq!(strip_metadata(&mime::TEXT_PLAIN, text.clone()), text);

        // A JPEG content type over data that isn't one.
        let not_a_jpeg = b"nope".to_vec();
        assert_eq!(strip_metadata(&mime::IMAGE_JPEG, not_a_jpeg.clone()), not_a_jpeg);

        // A truncated JPEG: better to send it as-is than to mangle it.
        let truncated = vec![0xFF, 0xD8, 0xFF, 0xE1, 0x00];
        assert_eq!(strip_metadata(&mime::IMAGE_JPEG, truncated.clone()), truncated);
    }

    /// Stripping an already stripped image changes nothing more.
    #[test]
    fn test_stripping_is_idempotent() {
        for (content_type, data) in [
            (mime::IMAGE_JPEG, jpeg_with_secrets(6)),
            (mime::IMAGE_PNG, png_with_secrets(3)),
            ("image/webp".parse().unwrap(), webp_with_secrets(8)),
        ] {
            let once = strip_metadata(&content_type, data);
            let twice = strip_metadata(&content_type, once.clone());
            assert_eq!(once, twice);
        }
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|window| window == needle)
    }

    /// Find the TIFF block of the one Exif `APP1` segment of a JPEG.
    fn read_orientation_of_jpeg(jpeg: &[u8]) -> Option<u16> {
        let mut pos = 2;

        while pos + 4 <= jpeg.len() {
            let marker = jpeg[pos + 1];
            let length = u16::from_be_bytes(jpeg[pos + 2..pos + 4].try_into().unwrap()) as usize;
            let payload = &jpeg[pos + 4..pos + 2 + length];

            if marker == 0xE1 && payload.starts_with(JPEG_EXIF_ID) {
                return read_orientation(&payload[JPEG_EXIF_ID.len()..]);
            }

            if marker == 0xDA {
                break;
            }

            pos += 2 + length;
        }

        None
    }

    fn png_chunk<'a>(png: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
        let mut pos = 8;

        while pos + 12 <= png.len() {
            let length = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
            let chunk_type = &png[pos + 4..pos + 8];
            let data = &png[pos + 8..pos + 8 + length];

            if chunk_type == wanted {
                // Check the CRC while we're here: a chunk we build ourselves
                // has to be readable by a real decoder.
                let crc = u32::from_be_bytes(
                    png[pos + 8 + length..pos + 12 + length].try_into().unwrap(),
                );
                assert_eq!(crc, super::png_crc32(&png[pos + 4..pos + 8 + length]));

                return Some(data);
            }

            pos += 12 + length;
        }

        None
    }

    fn webp_chunk_data<'a>(webp: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
        let mut pos = 12;

        while pos + 8 <= webp.len() {
            let chunk_type = &webp[pos..pos + 4];
            let length = u32::from_le_bytes(webp[pos + 4..pos + 8].try_into().unwrap()) as usize;

            if chunk_type == wanted {
                return Some(&webp[pos + 8..pos + 8 + length]);
            }

            pos += 8 + length + (length % 2);
        }

        None
    }
}
