// SPDX-License-Identifier: MIT

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A client discovery document served at `/.well-known/weft-client`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDocument {
    pub version: String,
    pub node: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
}

/// The node descriptor returned by `GET /api/v1/node`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeDescriptor {
    pub version: String,
    pub node_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub public_url: String,
    pub capabilities: Capabilities,
}

/// Public protocol capabilities advertised by a node.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(default)]
    pub api: Vec<String>,
    #[serde(default)]
    pub media: Vec<String>,
    #[serde(default)]
    pub replication: bool,
    #[serde(default)]
    pub federation: bool,
}

/// A page of list results. `next_cursor` is opaque to callers.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// One item in search, trending or media listings.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContentSummary {
    pub hash: String,
    pub content_id: String,
    pub mime: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub frames: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub licence: Licence,
    pub media_url: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    pub manifest_url: String,
    #[serde(default)]
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub status: String,
}

impl ContentSummary {
    /// True for formats that may carry animation.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        matches!(
            self.mime.to_ascii_lowercase().as_str(),
            "image/gif" | "image/webp" | "image/apng" | "image/png"
        ) && self.frames.is_some_and(|frames| frames > 1)
    }
}

/// Licence metadata attached to a summary.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct Licence {
    #[serde(default)]
    pub claim: Option<String>,
    #[serde(default)]
    pub spdx: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub status: String,
}

/// A tag with its local item count.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

/// Signed media manifest.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub version: String,
    pub content_id: String,
    pub hash: String,
    pub mime: String,
    pub size: i64,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub frames: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub origin: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub sources: Vec<Source>,
    pub licence: ManifestLicence,
    pub moderation: Moderation,
    #[serde(default)]
    pub tags: Vec<String>,
    pub signature: SourceSignature,
}

/// Licence details with evidence links in a manifest.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct ManifestLicence {
    #[serde(default)]
    pub claim: Option<String>,
    #[serde(default)]
    pub spdx: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// Local moderation state recorded in a manifest.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Moderation {
    pub state: String,
    pub updated_at: DateTime<Utc>,
}

/// One known source for the bytes.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub node_id: String,
    pub url: String,
    pub verified_at: DateTime<Utc>,
    pub bytes: i64,
}

/// Signed list of sources.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Sources {
    pub content_id: String,
    pub sources: Vec<Source>,
    pub signature: SourceSignature,
}

/// Ed25519 signature over the canonical object.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceSignature {
    pub alg: String,
    pub key_id: String,
    pub value: String,
}
