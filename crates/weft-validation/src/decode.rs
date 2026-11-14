// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decode media in a separate worker and produce a safe thumbnail.

use std::fs::{self, File};
use std::io::Cursor;
use std::path::Path;

use image::ImageReader;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{sniff, Limits, MediaFormat, ValidationError};

/// The result returned by the `weft-decode` worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedMedia {
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub duration_ms: u64,
}

/// Decode an input file, enforce limits and write a JPEG thumbnail.
pub fn decode_file(
    input: &Path,
    thumbnail: &Path,
    limits: Limits,
) -> Result<DecodedMedia, DecodeError> {
    let bytes = fs::read(input)?;
    if bytes.len() as u64 > limits.max_bytes {
        return Err(ValidationError::TooLarge {
            bytes: bytes.len() as u64,
            max: limits.max_bytes,
        }
        .into());
    }
    let format = sniff(&bytes)?;

    let (width, height, frames, duration_ms) = match format {
        MediaFormat::Gif => decode_gif(&bytes, limits)?,
        _ => decode_static(&bytes)?,
    };

    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > limits.max_pixels {
        return Err(DecodeError::TooManyPixels {
            pixels,
            max: limits.max_pixels,
        });
    }
    if frames > limits.max_frames {
        return Err(DecodeError::TooManyFrames {
            frames,
            max: limits.max_frames,
        });
    }
    let decoded = pixels
        .saturating_mul(4)
        .saturating_mul(u64::from(frames.max(1)));
    if decoded > limits.max_decoded_bytes {
        return Err(DecodeError::DecodedTooLarge {
            bytes: decoded,
            max: limits.max_decoded_bytes,
        });
    }

    let image = image::load_from_memory(&bytes)?;
    let thumbnail_image = image.thumbnail(512, 512).to_rgb8();
    let mut output = File::create(thumbnail)?;
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 82)
        .encode_image(&thumbnail_image)?;

    Ok(DecodedMedia {
        mime: format.mime().to_string(),
        width,
        height,
        frames,
        duration_ms,
    })
}

fn decode_static(bytes: &[u8]) -> Result<(u32, u32, u32, u64), DecodeError> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let (width, height) = reader.into_dimensions()?;
    Ok((width, height, 1, 0))
}

fn decode_gif(bytes: &[u8], limits: Limits) -> Result<(u32, u32, u32, u64), DecodeError> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::Indexed);
    let mut decoder = options.read_info(Cursor::new(bytes))?;
    let width = u32::from(decoder.width());
    let height = u32::from(decoder.height());
    let mut frames = 0_u32;
    let mut duration_ms = 0_u64;

    while let Some(frame) = decoder.read_next_frame()? {
        frames = frames.saturating_add(1);
        duration_ms = duration_ms.saturating_add(u64::from(frame.delay) * 10);
        if frames > limits.max_frames {
            return Err(DecodeError::TooManyFrames {
                frames,
                max: limits.max_frames,
            });
        }
    }

    Ok((width, height, frames.max(1), duration_ms))
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("storage I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image decode error: {0}")]
    Image(#[from] image::ImageError),
    #[error("GIF decode error: {0}")]
    Gif(#[from] gif::DecodingError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("image has {pixels} pixels, above the {max} limit")]
    TooManyPixels { pixels: u64, max: u64 },
    #[error("GIF has {frames} frames, above the {max} limit")]
    TooManyFrames { frames: u32, max: u32 },
    #[error("decoded image would use {bytes} bytes, above the {max} limit")]
    DecodedTooLarge { bytes: u64, max: u64 },
}

#[cfg(test)]
mod tests {
    use super::decode_file;
    use crate::Limits;
    use std::io::Write;

    #[test]
    fn decodes_a_png_and_writes_a_thumbnail() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.png");
        let thumbnail = directory.path().join("thumb.jpg");
        let image = image::RgbImage::from_pixel(8, 6, image::Rgb([10, 20, 30]));
        image.save(&input).unwrap();
        let decoded = decode_file(&input, &thumbnail, Limits::default()).unwrap();
        assert_eq!((decoded.width, decoded.height), (8, 6));
        assert_eq!(decoded.frames, 1);
        assert!(thumbnail.exists());
    }

    #[test]
    fn decodes_a_gif_and_counts_frames() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.gif");
        let thumbnail = directory.path().join("thumb.jpg");
        let mut file = std::fs::File::create(&input).unwrap();
        let mut encoder = gif::Encoder::new(&mut file, 4, 4, &[]).unwrap();
        let pixels = [255_u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
        for _ in 0..2 {
            let frame = gif::Frame::from_rgb(2, 2, &pixels);
            encoder.write_frame(&frame).unwrap();
        }
        drop(encoder);
        file.flush().unwrap();
        let decoded = decode_file(&input, &thumbnail, Limits::default()).unwrap();
        assert_eq!(decoded.frames, 2);
        assert_eq!(decoded.mime, "image/gif");
        assert!(thumbnail.exists());
    }
}
