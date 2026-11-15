// SPDX-License-Identifier: MIT

use thiserror::Error;

/// Errors returned by a Weft client request.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("base URL must use http or https")]
    UnsupportedScheme,
    #[error("invalid content hash: {0}")]
    InvalidHash(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("could not decode the node response: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("client discovery failed: {0}")]
    Discovery(String),
    #[error("invalid client discovery document: {0}")]
    InvalidDiscovery(String),
    #[error("node returned {status} ({code}): {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
        retry_after: Option<u64>,
    },
}

impl Error {
    /// HTTP status for an API error, when the node supplied one.
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Seconds to wait before retrying, from `Retry-After`.
    #[must_use]
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            Self::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// True for a missing or removed item.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self.status(), Some(404 | 410))
    }

    /// True when the node is rate limiting this client.
    #[must_use]
    pub fn is_rate_limited(&self) -> bool {
        self.status() == Some(429)
    }
}
