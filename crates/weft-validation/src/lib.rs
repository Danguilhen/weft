// SPDX-License-Identifier: AGPL-3.0-or-later

//! Upload format sniffing and limit types.
//!
//! Milestone 0 only classifies the container. Decode limits and thumbnails are
//! implemented with the `weft-decode` worker in Milestone 1.

use thiserror::Error;

/// Media formats accepted by the MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFormat {
    Gif,
    Png,
    Jpeg,
    Webp,
}

impl MediaFormat {
    #[must_use]
    pub fn mime(self) -> &'static str {
        match self {
            Self::Gif => "image/gif",
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }
}

/// Hard limits applied before and during decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: u64,
    pub max_pixels: u64,
    pub max_frames: u32,
    pub max_decoded_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 15 * 1024 * 1024,
            max_pixels: 16_000_000,
            max_frames: 300,
            max_decoded_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Detect a supported container from its leading bytes.
///
/// This is not a full validation: the decoder must still enforce limits and
/// reject malformed files.
pub fn sniff(bytes: &[u8]) -> Result<MediaFormat, ValidationError> {
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok(MediaFormat::Gif);
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(MediaFormat::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok(MediaFormat::Jpeg);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(MediaFormat::Webp);
    }
    Err(ValidationError::Unsupported)
}

/// Check a byte length against the configured limit.
pub fn check_size(bytes: u64, limits: Limits) -> Result<(), ValidationError> {
    if bytes > limits.max_bytes {
        return Err(ValidationError::TooLarge {
            bytes,
            max: limits.max_bytes,
        });
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("unsupported media format")]
    Unsupported,
    #[error("file is {bytes} bytes, above the {max}-byte limit")]
    TooLarge { bytes: u64, max: u64 },
}

#[cfg(test)]
mod tests {
    use super::{check_size, sniff, Limits, MediaFormat, ValidationError};

    #[test]
    fn detects_supported_headers() {
        assert_eq!(sniff(b"GIF89a..."), Ok(MediaFormat::Gif));
        assert_eq!(sniff(b"GIF87a..."), Ok(MediaFormat::Gif));
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\nrest"), Ok(MediaFormat::Png));
        assert_eq!(sniff(b"\xff\xd8\xffrest"), Ok(MediaFormat::Jpeg));
        assert_eq!(sniff(b"RIFF1234WEBPrest"), Ok(MediaFormat::Webp));
    }

    #[test]
    fn rejects_unknown_containers() {
        assert_eq!(sniff(b"<svg></svg>"), Err(ValidationError::Unsupported));
        assert_eq!(sniff(b""), Err(ValidationError::Unsupported));
    }

    #[test]
    fn enforces_the_size_limit() {
        let limits = Limits::default();
        assert!(check_size(limits.max_bytes, limits).is_ok());
        assert_eq!(
            check_size(limits.max_bytes + 1, limits),
            Err(ValidationError::TooLarge {
                bytes: limits.max_bytes + 1,
                max: limits.max_bytes
            })
        );
    }
}
