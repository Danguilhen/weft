// SPDX-License-Identifier: MIT

//! Content identifiers and media kinds.

use std::fmt;
use std::str::FromStr;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// A SHA-256 digest of original media bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobHash(String);

impl BlobHash {
    /// Parse a lowercase or uppercase 64-character hexadecimal digest.
    pub fn parse(value: &str) -> Result<Self, IdError> {
        let value = value.trim().to_ascii_lowercase();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(IdError::InvalidHash);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn to_content_id(&self) -> ContentId {
        ContentId(self.clone())
    }
}

impl fmt::Display for BlobHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for BlobHash {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// The public, immutable identity of a media object.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentId(BlobHash);

impl ContentId {
    #[must_use]
    pub fn from_hash(hash: BlobHash) -> Self {
        Self(hash)
    }

    #[must_use]
    pub fn hash(&self) -> &BlobHash {
        &self.0
    }

    pub fn parse(value: &str) -> Result<Self, IdError> {
        let hash = value
            .strip_prefix("urn:weft:sha256:")
            .ok_or(IdError::InvalidContentId)?;
        Ok(Self(BlobHash::parse(hash)?))
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "urn:weft:sha256:{}", self.0)
    }
}

/// Compute the content hash of a byte slice.
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> BlobHash {
    let digest = Sha256::digest(bytes);
    BlobHash(hex::encode(digest))
}

/// Media kinds accepted by the Weft MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Gif,
    Png,
    Jpeg,
    Webp,
}

impl MediaType {
    #[must_use]
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/gif" => Some(Self::Gif),
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            "image/webp" => Some(Self::Webp),
            _ => None,
        }
    }

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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdError {
    #[error("a content hash must be 64 hexadecimal characters")]
    InvalidHash,
    #[error("a content id must start with urn:weft:sha256:")]
    InvalidContentId,
}

#[cfg(test)]
mod tests {
    use super::{hash_bytes, BlobHash, ContentId};

    #[test]
    fn hashes_are_lowercase_hex() {
        let hash = hash_bytes(b"weft");
        assert_eq!(hash.as_str().len(), 64);
        assert!(hash.as_str().bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(hash.as_str().bytes().all(|byte| !byte.is_ascii_uppercase()));
    }

    #[test]
    fn content_ids_round_trip() {
        let hash = hash_bytes(b"weft");
        let id = ContentId::from_hash(hash.clone());
        assert_eq!(ContentId::parse(&id.to_string()).unwrap(), id);
        assert_eq!(id.hash(), &hash);
    }

    #[test]
    fn rejects_bad_hashes() {
        assert!(BlobHash::parse("abc").is_err());
        assert!(BlobHash::parse(&"g".repeat(64)).is_err());
        assert!(ContentId::parse("urn:weft:sha256:not-a-hash").is_err());
    }

    #[test]
    fn media_types_map_to_mimes() {
        assert_eq!(
            crate::MediaType::from_mime("image/gif").unwrap().mime(),
            "image/gif"
        );
        assert!(crate::MediaType::from_mime("text/html").is_none());
    }
}
