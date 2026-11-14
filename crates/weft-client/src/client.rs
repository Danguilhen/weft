// SPDX-License-Identifier: MIT

use std::time::Duration;

use reqwest::header::{HeaderMap, RETRY_AFTER};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use url::Url;

use crate::error::Error;
use crate::model::{ContentSummary, Manifest, NodeDescriptor, Page, Sources, TagCount};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_USER_AGENT: &str = concat!("weft-client/", env!("CARGO_PKG_VERSION"));
const MAX_LIMIT: u32 = 100;

/// Sort order accepted by search and media listings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Relevance,
    Newest,
    Trending,
}

impl Sort {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Newest => "newest",
            Self::Trending => "trending",
        }
    }
}

/// Trending window accepted by the node. The node may ignore it until ranking
/// windows are implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Day,
    Week,
    Month,
}

impl Window {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "24h",
            Self::Week => "7d",
            Self::Month => "30d",
        }
    }
}

/// Query for search, trending and media listings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListQuery {
    text: Option<String>,
    tags: Vec<String>,
    sort: Option<Sort>,
    limit: Option<u32>,
    cursor: Option<String>,
    window: Option<Window>,
}

impl ListQuery {
    /// An empty query.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A free-text search query.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new().with_text(text)
    }

    /// A newest-first list query.
    #[must_use]
    pub fn newest() -> Self {
        Self::new().with_sort(Sort::Newest)
    }

    /// A trending query with a window.
    #[must_use]
    pub fn trending(window: Window) -> Self {
        Self::new().with_sort(Sort::Trending).with_window(window)
    }

    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    #[must_use]
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn with_sort(mut self, sort: Sort) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Set the page size. Values are clamped to `1..=100`.
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit.clamp(1, MAX_LIMIT));
        self
    }

    #[must_use]
    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    #[must_use]
    pub fn with_window(mut self, window: Window) -> Self {
        self.window = Some(window);
        self
    }
}

/// Original bytes and useful headers returned by a media download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedMedia {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub digest: Option<String>,
}

/// A client for one Weft node.
#[derive(Debug, Clone)]
pub struct Client {
    base: Url,
    http: reqwest::Client,
}

impl Client {
    /// Create a client for a node origin, such as `https://gifs.example`.
    pub fn new(base_url: impl AsRef<str>) -> Result<Self, Error> {
        Self::builder(base_url).build()
    }

    /// Start a client with a custom timeout and user agent.
    pub fn builder(base_url: impl AsRef<str>) -> ClientBuilder {
        ClientBuilder {
            base: normalize_base(base_url.as_ref()),
            timeout: DEFAULT_TIMEOUT,
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }

    /// Fetch the node descriptor.
    pub async fn node(&self) -> Result<NodeDescriptor, Error> {
        self.get_json("api/v1/node").await
    }

    /// Search the local index.
    pub async fn search(&self, query: ListQuery) -> Result<Page<ContentSummary>, Error> {
        self.list("api/v1/search", &query).await
    }

    /// List trending content. The window is sent as a hint.
    pub async fn trending(&self, query: ListQuery) -> Result<Page<ContentSummary>, Error> {
        self.list("api/v1/trending", &query).await
    }

    /// List the newest content.
    pub async fn latest(&self, query: ListQuery) -> Result<Page<ContentSummary>, Error> {
        self.list("api/v1/media", &query).await
    }

    /// Fetch the local tag vocabulary.
    pub async fn tags(&self) -> Result<Vec<TagCount>, Error> {
        let page: Page<TagCount> = self.get_json("api/v1/tags").await?;
        Ok(page.items)
    }

    /// Fetch the signed manifest for a content hash.
    pub async fn manifest(&self, hash: &str) -> Result<Manifest, Error> {
        validate_hash(hash)?;
        self.get_json(&format!("api/v1/media/{hash}")).await
    }

    /// Fetch the signed list of known sources for a content hash.
    pub async fn sources(&self, hash: &str) -> Result<Sources, Error> {
        validate_hash(hash)?;
        self.get_json(&format!("api/v1/media/{hash}/sources")).await
    }

    /// Build the original-bytes URL for a content hash.
    pub fn media_url(&self, hash: &str) -> Result<Url, Error> {
        validate_hash(hash)?;
        self.url(&format!("media/{hash}"))
    }

    /// Download the original bytes for a content hash.
    pub async fn download(&self, hash: &str) -> Result<DownloadedMedia, Error> {
        let url = self.media_url(hash)?;
        let response = self.http.get(url).send().await?;
        let response = check_status(response).await?;
        let content_type = header_string(response.headers(), reqwest::header::CONTENT_TYPE);
        let etag = header_string(response.headers(), reqwest::header::ETAG);
        let digest = header_string(
            response.headers(),
            reqwest::header::HeaderName::from_static("digest"),
        );
        let bytes = response.bytes().await?.to_vec();
        Ok(DownloadedMedia {
            bytes,
            content_type,
            etag,
            digest,
        })
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let request = self.http.get(self.url(path)?);
        let response = check_status(request.send().await?).await?;
        Ok(response.json().await?)
    }

    async fn list<T: DeserializeOwned>(&self, path: &str, query: &ListQuery) -> Result<T, Error> {
        let mut url = self.url(path)?;
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(text) = query.text.as_deref().filter(|text| !text.is_empty()) {
                pairs.append_pair("q", text);
            }
            if !query.tags.is_empty() {
                pairs.append_pair("tags", &query.tags.join(","));
            }
            if let Some(sort) = query.sort {
                pairs.append_pair("sort", sort.as_str());
            }
            if let Some(limit) = query.limit {
                pairs.append_pair("limit", &limit.to_string());
            }
            if let Some(cursor) = query.cursor.as_deref() {
                pairs.append_pair("cursor", cursor);
            }
            if let Some(window) = query.window {
                pairs.append_pair("window", window.as_str());
            }
        }
        let request = self.http.get(url);
        let response = check_status(request.send().await?).await?;
        Ok(response.json().await?)
    }

    fn url(&self, path: &str) -> Result<Url, Error> {
        Ok(self.base.join(path)?)
    }
}

/// Builder for [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base: String,
    timeout: Duration,
    user_agent: String,
}

impl ClientBuilder {
    /// Set the total request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the `User-Agent` header.
    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client, Error> {
        let base = normalize_base(&self.base);
        let mut base = Url::parse(&base)?;
        if !matches!(base.scheme(), "http" | "https") {
            return Err(Error::UnsupportedScheme);
        }
        base.set_query(None);
        base.set_fragment(None);
        let path = format!("{}/", base.path().trim_end_matches('/'));
        base.set_path(&path);
        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .user_agent(self.user_agent)
            .build()?;
        Ok(Client { base, http })
    }
}

fn normalize_base(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn validate_hash(hash: &str) -> Result<(), Error> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Error::InvalidHash(hash.to_string()))
    }
}

fn header_string(headers: &HeaderMap, name: reqwest::header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, Error> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok());
    let body = response.bytes().await.unwrap_or_default();
    let envelope = serde_json::from_slice::<ErrorEnvelope>(&body).ok();
    let (code, message) = envelope.map_or_else(
        || {
            (
                "http_error".to_string(),
                status
                    .canonical_reason()
                    .unwrap_or("request failed")
                    .to_string(),
            )
        },
        |envelope| (envelope.error.code, envelope.error.message),
    );
    Err(Error::Api {
        status: status.as_u16(),
        code,
        message,
        retry_after,
    })
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}
