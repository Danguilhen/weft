// SPDX-License-Identifier: AGPL-3.0-or-later

//! Weft HTTP server.

pub mod decode;
pub mod metrics;
pub mod rate_limit;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{
    ConnectInfo, DefaultBodyLimit, FromRequestParts, Multipart, Path as AxumPath, Query, Request,
    State,
};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tracing_subscriber::EnvFilter;
use url::Url;
use uuid::Uuid;
use weft_core::config::{AppConfig, LoggingConfig};
use weft_core::{hash_bytes, parse_size, BlobHash, ContentId, NodeSigner};
use weft_db::{self, BlockRule, ContentQuery, ContentRow, DbError, NewUpload, RecordedUpload};
use weft_storage::{BlobStore, LocalStore, StorageError};

use crate::decode::DecodeWorkerError;
use crate::metrics::Metrics;
use crate::rate_limit::{client_identity, ClientHasher, RateLimiter, TrustedProxies};

const OPENAPI_JSON: &str = include_str!("../../../docs/openapi.json");

/// Optional peer address supplied by `into_make_service_with_connect_info`.
#[derive(Clone, Copy, Debug)]
struct PeerAddr(Option<SocketAddr>);

impl<S> FromRequestParts<S> for PeerAddr
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|info| info.0),
        ))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub search: Arc<dyn weft_db::SearchIndex>,
    pub storage: Arc<dyn BlobStore>,
    pub metrics: Arc<Metrics>,
    pub node: Arc<NodePublic>,
    pub config: Arc<AppConfig>,
    pub signer: Arc<NodeSigner>,
    pub worker_path: PathBuf,
    pub rate: Arc<RateLimiter>,
    pub client_hasher: Arc<ClientHasher>,
    pub trusted_proxies: TrustedProxies,
    pub decode_semaphore: Arc<Semaphore>,
    pub max_upload_bytes: u64,
    pub max_daily_bytes: i64,
    pub max_json_bytes: usize,
    pub metrics_enabled: bool,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        storage: Arc<dyn BlobStore>,
        metrics: Arc<Metrics>,
        node: NodePublic,
        config: AppConfig,
        signer: NodeSigner,
    ) -> Self {
        let worker_path = decode::resolve_worker(&config.decode);
        let rate = Arc::new(RateLimiter::new(
            config.upload.rate_limit_per_hour,
            config.upload.rate_limit_burst,
        ));
        let max_upload_bytes =
            parse_size(&config.storage.max_file_size).unwrap_or(15 * 1024 * 1024);
        let max_daily_bytes =
            parse_size(&config.upload.max_daily_bytes).unwrap_or(100 * 1024 * 1024) as i64;
        let trusted_proxies = TrustedProxies::parse(&config.server.trusted_proxies)
            .expect("server.trusted_proxies is validated at load time");
        let client_hasher = Arc::new(if Path::new(&config.privacy.hash_salt_file).is_relative() {
            ClientHasher::load_or_create("").expect("ephemeral salt")
        } else {
            ClientHasher::load_or_create(&config.privacy.hash_salt_file).unwrap_or_else(|error| {
                tracing::warn!(%error, "could not load the client hash salt; using an ephemeral one");
                ClientHasher::load_or_create("").expect("ephemeral salt")
            })
        });
        let decode_semaphore = Arc::new(Semaphore::new(
            config.upload.max_concurrent_decodes.max(1) as usize,
        ));
        let search = Arc::new(weft_db::PostgresSearch::new(pool.clone()));
        let metrics_enabled = config.metrics.enabled;
        Self {
            pool,
            search,
            storage,
            metrics,
            node: Arc::new(node),
            max_json_bytes: config.server.max_json_bytes,
            config: Arc::new(config),
            signer: Arc::new(signer),
            worker_path,
            rate,
            client_hasher,
            trusted_proxies,
            decode_semaphore,
            max_upload_bytes,
            max_daily_bytes,
            metrics_enabled,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NodePublic {
    pub version: &'static str,
    #[serde(rename = "nodeId")]
    pub node_id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "publicUrl")]
    pub public_url: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Capabilities {
    pub api: Vec<&'static str>,
    pub media: Vec<&'static str>,
    pub replication: bool,
    pub federation: bool,
}

impl NodePublic {
    #[must_use]
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            node_id: config.node.actor_url(),
            name: config.node.name.clone(),
            description: config.node.description.clone(),
            public_url: config.node.public_url.clone(),
            capabilities: Capabilities {
                api: vec!["v1"],
                media: vec!["weft-wmp/0.1"],
                replication: false,
                federation: false,
            },
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    let metrics_enabled = state.metrics_enabled;
    let max_body = state
        .max_json_bytes
        .max(state.max_upload_bytes as usize + 1024 * 1024);
    let middleware_state = state.clone();

    let mut router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/node", get(node))
        .route("/api/v1/search", get(search))
        .route("/api/v1/trending", get(trending))
        .route("/api/v1/media", get(list_media))
        .route("/api/v1/tags", get(tags))
        .route("/api/v1/media/{hash}", get(manifest))
        .route("/api/v1/media/{hash}/sources", get(sources))
        .route("/media/{hash}", get(media).head(media_head))
        .route("/media/{hash}/thumbnail", get(thumbnail))
        .route("/media/{hash}/manifest", get(manifest))
        .route("/media/{hash}/sources", get(sources))
        .route("/api/v1/upload", post(upload))
        .route("/api/v1/report", post(report))
        .route("/api/v1/licence-dispute", post(licence_dispute))
        .route("/api/v1/admin/queue", get(admin_queue))
        .route("/api/v1/admin/content/{hash}/{action}", post(admin_action))
        .route("/api/v1/admin/content/{hash}/licence", post(admin_licence))
        .route("/api/v1/admin/reports", get(admin_reports))
        .route(
            "/api/v1/admin/storage",
            get(admin_storage).post(admin_storage_cleanup),
        )
        .route(
            "/api/v1/admin/blocks",
            get(admin_blocks).post(add_admin_block),
        )
        .route("/api/v1/admin/blocks/{id}", delete(delete_admin_block))
        .route("/api/v1/admin/stats", get(admin_stats))
        .route("/api/openapi.json", get(openapi));

    if metrics_enabled {
        router = router.route("/metrics", get(metrics));
    }

    router
        .layer(middleware::from_fn_with_state(
            middleware_state,
            request_logging,
        ))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}

/// Run the node until Ctrl-C.
pub async fn run(config: AppConfig) -> Result<(), ServerError> {
    init_logging(&config.logging);
    tracing::info!(bind = %config.server.bind, "starting Weft");

    let pool = weft_db::connect(&config.database).await?;
    weft_db::migrate(&pool).await?;
    weft_db::ensure_node(&pool, &config.node).await?;

    let key_id = format!("{}#key-1", config.node.actor_url());
    let signer = NodeSigner::load_or_create(&config.node.key_file, key_id)?;
    let storage = Arc::new(LocalStore::new(&config.storage.path)?);
    let state = AppState::new(
        pool,
        storage,
        Arc::new(Metrics::default()),
        NodePublic::from_config(&config),
        config.clone(),
        signer,
    );
    let app = build_router(state.clone());
    spawn_maintenance(state);

    let listener = TcpListener::bind(&config.server.bind).await?;
    tracing::info!(bind = %config.server.bind, "listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

fn init_logging(config: &LoggingConfig) {
    let filter = EnvFilter::try_new(&config.level).unwrap_or_else(|_| EnvFilter::new("info"));
    if config.format == "json" {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown requested");
}

async fn healthz() -> &'static str {
    "ok"
}

#[derive(serde::Serialize)]
struct Ready {
    status: &'static str,
    database: &'static str,
    storage: &'static str,
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let database = weft_db::health(&state.pool).await;
    let storage = state.storage.check();

    if database.is_err() || storage.is_err() {
        if let Err(error) = &database {
            tracing::warn!(%error, "readiness: database unavailable");
        }
        if let Err(error) = &storage {
            tracing::warn!(%error, "readiness: storage unavailable");
        }
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(Ready {
                status: "unavailable",
                database: if database.is_ok() {
                    "ok"
                } else {
                    "unavailable"
                },
                storage: if storage.is_ok() { "ok" } else { "unavailable" },
            }),
        );
    }

    (
        StatusCode::OK,
        Json(Ready {
            status: "ready",
            database: "ok",
            storage: "ok",
        }),
    )
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    (
        [("content-type", "text/plain; version=0.0.4")],
        state.metrics.render(),
    )
}

async fn node(State(state): State<AppState>) -> Json<NodePublic> {
    Json((*state.node).clone())
}

#[derive(Debug, Default, Deserialize)]
struct ListParams {
    q: Option<String>,
    tags: Option<String>,
    tag: Option<String>,
    sort: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
    #[allow(dead_code)]
    window: Option<String>,
}

async fn search(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    list_response(&state, params, "relevance").await
}

async fn trending(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    list_response(&state, params, "trending").await
}

async fn list_media(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    list_response(&state, params, "newest").await
}

async fn list_response(
    state: &AppState,
    params: ListParams,
    default_sort: &str,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = decode_cursor(params.cursor.as_deref())?;
    let mut tags = split_tags(params.tags.as_deref());
    tags.extend(split_tags(params.tag.as_deref()));
    tags.sort();
    tags.dedup();
    let sort = params.sort.unwrap_or_else(|| default_sort.to_string());
    let rules = weft_db::block_rules(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    let blocked_tags: Vec<String> = rules
        .iter()
        .filter(|rule| rule.kind == "tag")
        .map(|rule| rule.value.to_ascii_lowercase())
        .collect();
    let query = ContentQuery {
        text: params.q.unwrap_or_default(),
        tags,
        blocked_tags,
        sort,
        limit,
        offset,
    };
    let rows = state
        .search
        .search(&query)
        .await
        .map_err(ApiError::internal)?;
    let items: Vec<Value> = rows.iter().map(|row| summary(row, state)).collect();
    let next = (items.len() as i64 == limit).then(|| encode_cursor(offset + limit));
    Ok(Json(json!({ "items": items, "nextCursor": next })))
}

async fn tags(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows = weft_db::tags(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    let items: Vec<Value> = rows
        .iter()
        .map(|row| json!({ "tag": row.tag, "count": row.count }))
        .collect();
    Ok(Json(json!({ "items": items, "nextCursor": Value::Null })))
}

async fn manifest(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let row = weft_db::get_content(&state.pool, &hash, false)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::not_found(
                "content_not_found",
                "No active content exists for this hash.",
            )
        })?;
    ensure_not_blocked(&state, &row).await?;
    Ok(Json(manifest_json(&state, &row)?))
}

async fn sources(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<Value>, ApiError> {
    let row = weft_db::get_content(&state.pool, &hash, false)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::not_found(
                "content_not_found",
                "No active content exists for this hash.",
            )
        })?;
    ensure_not_blocked(&state, &row).await?;
    let mut value = json!({
        "contentId": content_id(&row)?,
        "sources": [source_value(&state, &row)]
    });
    let signature = state
        .signer
        .sign_value(&value)
        .map_err(ApiError::internal)?;
    value["signature"] = serde_json::to_value(signature).map_err(ApiError::internal)?;
    Ok(Json(value))
}

async fn media(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    media_response(&state, &hash, &headers, false).await
}

async fn media_head(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    media_response(&state, &hash, &headers, true).await
}

async fn thumbnail(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Response, ApiError> {
    let row = weft_db::get_content(&state.pool, &hash, false)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::not_found(
                "content_not_found",
                "No active content exists for this hash.",
            )
        })?;
    ensure_not_blocked(&state, &row).await?;
    let key = row.thumbnail_key.ok_or_else(|| {
        ApiError::not_found(
            "thumbnail_not_found",
            "No thumbnail exists for this content.",
        )
    })?;
    let bytes = state
        .storage
        .get_thumbnail(&key)
        .map_err(|error| match error {
            StorageError::NotFound(_) => ApiError::not_found(
                "thumbnail_not_found",
                "The thumbnail is missing from storage.",
            ),
            other => ApiError::internal(other),
        })?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::ETAG, format!("\"thumb-{hash}\""))
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(bytes))
        .map_err(ApiError::internal)
}

async fn media_response(
    state: &AppState,
    hash: &str,
    headers: &HeaderMap,
    head_only: bool,
) -> Result<Response, ApiError> {
    let row = weft_db::get_content(&state.pool, hash, false)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::not_found(
                "content_not_found",
                "No active content exists for this hash.",
            )
        })?;
    ensure_not_blocked(state, &row).await?;
    let blob_hash = BlobHash::parse(&row.hash).map_err(ApiError::internal)?;
    let bytes = state.storage.get(&blob_hash).map_err(|error| match error {
        StorageError::NotFound(_) => {
            ApiError::not_found("content_not_found", "The blob is missing from storage.")
        }
        other => {
            state.metrics.upload_failed();
            ApiError::internal(other)
        }
    })?;

    let total = bytes.len();
    let range = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| parse_range(value, total));

    if total == 0 {
        return Err(ApiError::not_found(
            "content_not_found",
            "The stored blob is empty.",
        ));
    }

    let (status, start, end) = match range {
        Some((start, end)) if start <= end && end < total => {
            (StatusCode::PARTIAL_CONTENT, start, end)
        }
        Some(_) => {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .body(Body::empty())
                .map_err(ApiError::internal);
        }
        None => (StatusCode::OK, 0, total.saturating_sub(1)),
    };

    let length = if total == 0 { 0 } else { end - start + 1 };
    let body = if head_only {
        Body::empty()
    } else {
        Body::from(bytes[start..=end.min(total.saturating_sub(1))].to_vec())
    };
    let digest = Sha256::digest(&bytes);
    let digest = base64::engine::general_purpose::STANDARD.encode(digest);
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, &row.mime)
        .header(header::CONTENT_LENGTH, length)
        .header(header::ETAG, format!("\"sha256:{}\"", row.hash))
        .header(header::ACCEPT_RANGES, "bytes")
        .header("digest", format!("sha-256=:{digest}:"))
        .header("x-content-type-options", "nosniff")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
    if status == StatusCode::PARTIAL_CONTENT {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total}"),
        );
    }
    if !head_only {
        state.metrics.media_served(length as u64);
        if let Err(error) = weft_db::increment_views(&state.pool, &row.hash).await {
            tracing::warn!(%error, "could not increment views");
        }
    }
    builder.body(body).map_err(ApiError::internal)
}

fn parse_range(value: &str, total: usize) -> Option<(usize, usize)> {
    let value = value.strip_prefix("bytes=")?.split(',').next()?;
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix: usize = end.parse().ok()?;
        if suffix == 0 || total == 0 {
            return None;
        }
        return Some((total.saturating_sub(suffix), total - 1));
    }
    let start: usize = start.parse().ok()?;
    let end = if end.is_empty() {
        total.checked_sub(1)?
    } else {
        end.parse().ok()?
    };
    (start < total).then_some((start, end.min(total - 1)))
}

#[derive(Debug, Default)]
struct UploadFields {
    title: String,
    description: String,
    tags: String,
    source: String,
    licence: String,
    licence_url: String,
    evidence: String,
    file: Option<Vec<u8>>,
}

fn enforce_quotas(
    config: &weft_core::config::UploadConfig,
    pending_total: i64,
    pending_client: i64,
    daily_bytes: i64,
    incoming_bytes: i64,
) -> Result<(), ApiError> {
    if pending_total >= i64::from(config.max_pending_total) {
        return Err(ApiError::quota(
            StatusCode::SERVICE_UNAVAILABLE,
            "quarantine_full",
            "The review queue is full; try again later.",
            3600,
        ));
    }
    if pending_client >= i64::from(config.max_pending_per_client) {
        return Err(ApiError::quota(
            StatusCode::TOO_MANY_REQUESTS,
            "pending_quota",
            "You have too many uploads waiting for review.",
            3600,
        ));
    }
    let max_daily = parse_size(&config.max_daily_bytes).unwrap_or(100 * 1024 * 1024) as i64;
    if daily_bytes.saturating_add(incoming_bytes) > max_daily {
        return Err(ApiError::quota(
            StatusCode::TOO_MANY_REQUESTS,
            "daily_quota",
            "The daily upload byte quota is exhausted.",
            3600,
        ));
    }
    Ok(())
}

async fn upload(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if state.config.upload.mode == "closed" {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "This node does not accept uploads.",
        ));
    }
    let client = client_identity(
        &headers,
        peer.0.map(|address| address.ip()),
        &state.trusted_proxies,
        state.config.upload.rate_limit_ipv6_prefix,
    );
    if let Err(retry) = state.rate.check(&client) {
        return Err(ApiError::rate_limited(retry));
    }
    let client_digest = state.client_hasher.digest(&client);
    let client_hash = &client_digest[..16];

    let mut fields = UploadFields::default();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad("invalid_multipart", error.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            let bytes = field
                .bytes()
                .await
                .map_err(|error| ApiError::bad("invalid_upload", error.to_string()))?;
            if bytes.len() as u64 > state.max_upload_bytes {
                return Err(ApiError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "payload_too_large",
                    "The file exceeds the configured upload limit.",
                ));
            }
            fields.file = Some(bytes.to_vec());
        } else {
            let value = field
                .text()
                .await
                .map_err(|error| ApiError::bad("invalid_field", error.to_string()))?;
            match name.as_str() {
                "title" => fields.title = value,
                "description" => fields.description = value,
                "tags" => fields.tags = value,
                "source" => fields.source = value,
                "licence" => fields.licence = value,
                "licenceUrl" => fields.licence_url = value,
                "evidence" => {
                    if !fields.evidence.is_empty() {
                        fields.evidence.push(',');
                    }
                    fields.evidence.push_str(&value);
                }
                _ => {}
            }
        }
    }

    let bytes = fields
        .file
        .ok_or_else(|| ApiError::bad("missing_file", "The `file` field is required."))?;
    let format = weft_validation::sniff(&bytes).map_err(|_| {
        state.metrics.upload_failed();
        ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "Only GIF, PNG, JPEG and WebP are accepted.",
        )
    })?;

    if let Some(max_size) = parse_size(&state.config.storage.max_size) {
        let report = weft_db::storage_report(&state.pool)
            .await
            .map_err(ApiError::internal)?;
        if report.bytes.saturating_add(bytes.len() as i64) > max_size as i64 {
            state.metrics.upload_failed();
            return Err(ApiError::new(
                StatusCode::INSUFFICIENT_STORAGE,
                "storage_full",
                "The node storage budget is exhausted.",
            ));
        }
    }

    let pending_total = weft_db::pending_count(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    let pending_client = weft_db::pending_count_for_client(&state.pool, client_hash)
        .await
        .map_err(ApiError::internal)?;
    let daily_bytes = weft_db::uploaded_bytes_since(&state.pool, client_hash, 24)
        .await
        .map_err(ApiError::internal)?;
    if let Err(error) = enforce_quotas(
        &state.config.upload,
        pending_total,
        pending_client,
        daily_bytes,
        bytes.len() as i64,
    ) {
        state.metrics.quota_rejected();
        state.metrics.upload_failed();
        return Err(error);
    }

    if fields.title.chars().count() > state.config.upload.max_title_chars as usize {
        return Err(ApiError::bad("title_too_long", "The title is too long."));
    }
    if fields.description.chars().count() > state.config.upload.max_description_chars as usize {
        return Err(ApiError::bad(
            "description_too_long",
            "The description is too long.",
        ));
    }
    let tags = normalize_tags(
        &fields.tags,
        state.config.upload.max_tags as usize,
        state.config.upload.max_tag_chars as usize,
    );
    let evidence: Vec<String> = fields
        .evidence
        .split(',')
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .collect();
    let rules = weft_db::block_rules(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    let source = (!fields.source.trim().is_empty()).then_some(fields.source.trim());
    if let Some(rule) = blocked_by(&rules, &tags, source) {
        state.metrics.upload_failed();
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "blocked",
            format!("This node blocks {rule}."),
        ));
    }
    let licence_status = if evidence.is_empty() {
        "unverified"
    } else {
        "evidence_provided"
    };

    let directory = tempfile::tempdir().map_err(ApiError::internal)?;
    let input = directory.path().join("upload.bin");
    let thumbnail_path = directory.path().join("thumb.jpg");
    std::fs::write(&input, &bytes).map_err(ApiError::internal)?;
    let permit = state.decode_semaphore.try_acquire_owned().map_err(|_| {
        state.metrics.decode_busy();
        state.metrics.upload_failed();
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "decode_busy",
            "The decode queue is full; retry shortly.",
        )
        .with_retry(5)
    })?;
    let decoded = decode::run_worker(
        &state.worker_path,
        &input,
        &thumbnail_path,
        &state.config.upload,
        &state.config.decode,
        state.max_upload_bytes,
        state.config.storage.thumbnail_size,
    )
    .await
    .map_err(|error| match error {
        DecodeWorkerError::Rejected(detail) => {
            state.metrics.decode_failed();
            state.metrics.upload_failed();
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                detail,
            )
        }
        DecodeWorkerError::Timeout => {
            state.metrics.decode_failed();
            state.metrics.upload_failed();
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Media decoding timed out.",
            )
        }
        other => {
            state.metrics.upload_failed();
            ApiError::internal(other)
        }
    })?;

    drop(permit);
    let hash = hash_bytes(&bytes);
    state
        .storage
        .put(&hash, &bytes)
        .map_err(ApiError::internal)?;
    let thumbnail = std::fs::read(&thumbnail_path).map_err(ApiError::internal)?;
    let thumbnail_key = state
        .storage
        .put_thumbnail(&hash, &thumbnail)
        .map_err(ApiError::internal)?;

    let is_open = state.config.upload.mode == "open";
    let status = if is_open { "active" } else { "pending" };
    let review = if is_open { "not_required" } else { "queued" };
    let recorded: RecordedUpload = weft_db::record_upload(
        &state.pool,
        &NewUpload {
            hash: hash.as_str(),
            mime: decoded.mime.as_str(),
            size: bytes.len() as i64,
            width: Some(decoded.width as i32),
            height: Some(decoded.height as i32),
            frames: Some(decoded.frames as i32),
            duration_ms: Some(decoded.duration_ms as i32),
            storage_key: hash.as_str(),
            thumbnail_key: Some(&thumbnail_key),
            title: &fields.title,
            description: &fields.description,
            origin_node: &state.config.node.actor_url(),
            status,
            licence_spdx: (!fields.licence.is_empty()).then_some(fields.licence.as_str()),
            licence_url: (!fields.licence_url.is_empty()).then_some(fields.licence_url.as_str()),
            licence_claim: (!fields.licence.is_empty()).then_some(fields.licence.as_str()),
            licence_status,
            source_url: (!fields.source.trim().is_empty()).then_some(fields.source.trim()),
            licence_evidence: &evidence,
            tags: &tags,
            review,
            client_hash: Some(client_hash),
        },
    )
    .await
    .map_err(ApiError::internal)?;

    // The worker reports the decoded MIME, but the bytes are authoritative.
    if decoded.mime != format.mime() {
        tracing::warn!(hash = %hash, "decoder and server MIME disagreement");
    }

    state.metrics.upload_accepted();
    state.metrics.stored(bytes.len() as u64);
    let content_id = hash.to_content_id().to_string();
    let manifest_url = format!(
        "{}/api/v1/media/{}",
        state.config.server.external_url.trim_end_matches('/'),
        hash
    );
    let response_status = recorded.content_status.clone();
    let response_review = if response_status == "active" {
        "not_required"
    } else {
        review
    };
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({
            "uploadId": recorded.upload_id.to_string(),
            "contentId": content_id,
            "hash": hash.as_str(),
            "status": response_status,
            "review": response_review,
            "manifestUrl": manifest_url,
        })),
    ))
}

#[derive(Debug, Deserialize)]
struct ReportRequest {
    #[serde(rename = "contentId")]
    content_id: String,
    reason: String,
    details: Option<String>,
    contact: Option<String>,
}

async fn report(
    State(state): State<AppState>,
    Json(request): Json<ReportRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let hash = parse_content_id(&request.content_id)?;
    if !matches!(
        request.reason.as_str(),
        "licence" | "illegal" | "spam" | "malware" | "privacy" | "other"
    ) {
        return Err(ApiError::bad("invalid_reason", "Unknown report reason."));
    }
    let contact_hash = request
        .contact
        .as_ref()
        .map(|contact| state.client_hasher.digest(contact)[..16].to_vec());
    let report_id = weft_db::insert_report(
        &state.pool,
        &hash,
        &request.reason,
        request.details.as_deref().unwrap_or_default(),
        contact_hash.as_deref(),
    )
    .await
    .map_err(ApiError::internal)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "reportId": report_id.to_string() })),
    ))
}

#[derive(Debug, Deserialize)]
struct DisputeRequest {
    #[serde(rename = "contentId")]
    content_id: String,
    claim: String,
    #[serde(default)]
    evidence: Vec<String>,
    contact: Option<String>,
}

async fn licence_dispute(
    State(state): State<AppState>,
    Json(request): Json<DisputeRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let hash = parse_content_id(&request.content_id)?;
    if request.claim.trim().is_empty() {
        return Err(ApiError::bad("invalid_claim", "A claim is required."));
    }
    let contact_hash = request
        .contact
        .as_ref()
        .map(|contact| state.client_hasher.digest(contact)[..16].to_vec());
    let report_id = weft_db::insert_licence_dispute(
        &state.pool,
        &hash,
        &request.claim,
        &request.evidence,
        contact_hash.as_deref(),
    )
    .await
    .map_err(ApiError::internal)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "reportId": report_id.to_string() })),
    ))
}

async fn admin_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = decode_cursor(params.cursor.as_deref())?;
    let rows = weft_db::review_queue(&state.pool, limit, offset)
        .await
        .map_err(ApiError::internal)?;
    let items: Vec<Value> = rows.iter().map(|row| admin_summary(row, &state)).collect();
    let next = (items.len() as i64 == limit).then(|| encode_cursor(offset + limit));
    Ok(Json(json!({ "items": items, "nextCursor": next })))
}

#[derive(Debug, Deserialize)]
struct LicenceUpdate {
    status: String,
    #[serde(default)]
    note: String,
}

async fn admin_licence(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(hash): AxumPath<String>,
    Json(request): Json<LicenceUpdate>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    if !matches!(
        request.status.as_str(),
        "unverified" | "evidence_provided" | "verified_evidence" | "disputed" | "invalidated"
    ) {
        return Err(ApiError::bad(
            "invalid_licence_status",
            "Unknown licence status.",
        ));
    }
    let row =
        weft_db::set_licence_status(&state.pool, &hash, &request.status, &request.note, "admin")
            .await
            .map_err(ApiError::internal)?
            .ok_or_else(|| {
                ApiError::not_found("content_not_found", "No content exists for this hash.")
            })?;
    Ok(Json(json!({
        "hash": row.hash,
        "licenceStatus": row.licence_status,
        "updatedAt": row.updated_at.to_rfc3339(),
    })))
}

async fn admin_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath((hash, action)): AxumPath<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let status = match action.as_str() {
        "approve" | "reinstate" => "active",
        "reject" => "rejected",
        "remove" => "removed",
        "restrict" => "restricted",
        _ => {
            return Err(ApiError::bad(
                "invalid_action",
                "Expected approve, reject, remove, restrict or reinstate.",
            ));
        }
    };
    let reason = format!("admin_{action}");
    let row = weft_db::set_status(&state.pool, &hash, status, &reason, "admin")
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::not_found("content_not_found", "No content exists for this hash.")
        })?;

    if matches!(status, "rejected" | "removed") {
        let orphan = weft_db::delete_blob_if_orphan(&state.pool, &hash)
            .await
            .map_err(ApiError::internal)?;
        if orphan {
            let blob_hash = BlobHash::parse(&hash).map_err(ApiError::internal)?;
            let _ = state.storage.delete(&blob_hash);
            if let Some(key) = &row.thumbnail_key {
                let _ = state.storage.delete_thumbnail(key);
            }
        }
    }

    Ok(Json(json!({
        "hash": row.hash,
        "status": row.status,
        "updatedAt": row.updated_at.to_rfc3339(),
    })))
}

async fn admin_reports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let reports = weft_db::list_reports(&state.pool, 100)
        .await
        .map_err(ApiError::internal)?;
    let items: Vec<Value> = reports
        .iter()
        .map(|report| {
            json!({
                "id": report.id.to_string(),
                "hash": report.hash,
                "reason": report.reason,
                "details": report.details,
                "status": report.status,
                "createdAt": report.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "items": items })))
}

async fn admin_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let stats = weft_db::admin_stats(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({
        "active": stats.active,
        "pending": stats.pending,
        "removed": stats.removed,
        "bytes": stats.bytes,
    })))
}

async fn ensure_not_blocked(state: &AppState, row: &ContentRow) -> Result<(), ApiError> {
    let rules = weft_db::block_rules(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    if blocked_by(&rules, &row.tags, row.source_url.as_deref()).is_some() {
        return Err(ApiError::not_found(
            "content_not_found",
            "No active content exists for this hash.",
        ));
    }
    Ok(())
}

fn blocked_by(rules: &[BlockRule], tags: &[String], source: Option<&str>) -> Option<String> {
    let host = source
        .and_then(|value| Url::parse(value).ok())
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase));
    for rule in rules {
        let matched = match rule.kind.as_str() {
            "tag" => tags.iter().any(|tag| tag.eq_ignore_ascii_case(&rule.value)),
            "domain" => host.as_deref().is_some_and(|host| {
                host == rule.value || host.ends_with(&format!(".{}", rule.value))
            }),
            "url_prefix" => source.is_some_and(|source| source.starts_with(&rule.value)),
            _ => false,
        };
        if matched {
            return Some(format!("{} {}", rule.kind, rule.value));
        }
    }
    None
}

async fn admin_blocks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let rules = weft_db::block_rules(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    let items: Vec<Value> = rules
        .iter()
        .map(|rule| {
            json!({
                "id": rule.id.to_string(),
                "kind": rule.kind,
                "value": rule.value,
                "reason": rule.reason,
                "createdAt": rule.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "items": items })))
}

#[derive(Debug, Deserialize)]
struct BlockRequest {
    kind: String,
    value: String,
    reason: Option<String>,
}

async fn add_admin_block(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BlockRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    require_admin(&state, &headers)?;
    if !matches!(
        request.kind.as_str(),
        "node" | "domain" | "url_prefix" | "tag"
    ) {
        return Err(ApiError::bad(
            "invalid_block_kind",
            "kind must be node, domain, url_prefix or tag.",
        ));
    }
    let value = request.value.trim();
    if value.is_empty() {
        return Err(ApiError::bad("invalid_block_value", "A value is required."));
    }
    let value = if matches!(request.kind.as_str(), "domain" | "tag") {
        value.to_ascii_lowercase()
    } else {
        value.to_string()
    };
    let id = weft_db::add_block(
        &state.pool,
        &request.kind,
        &value,
        request.reason.as_deref().unwrap_or_default(),
    )
    .await
    .map_err(ApiError::internal)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id.to_string() }))))
}

async fn delete_admin_block(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers)?;
    let id =
        Uuid::parse_str(&id).map_err(|_| ApiError::bad("invalid_block_id", "Invalid block id."))?;
    if weft_db::delete_block(&state.pool, id)
        .await
        .map_err(ApiError::internal)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(
            "block_not_found",
            "No such block rule.",
        ))
    }
}

fn spawn_maintenance(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60));
        loop {
            interval.tick().await;
            if let Err(error) = run_maintenance(&state).await {
                tracing::warn!(%error, "maintenance pass failed");
            }
        }
    });
}

async fn run_maintenance(state: &AppState) -> Result<(), DbError> {
    let expired = weft_db::expire_pending(&state.pool, state.config.upload.quarantine_days).await?;
    state.metrics.quarantine_expired(expired.len() as u64);

    let (clients, reports) = weft_db::retention_cleanup(
        &state.pool,
        state.config.privacy.client_hash_retention_days,
        state.config.privacy.report_retention_days,
    )
    .await?;
    state.metrics.retention_purged(clients + reports);

    let (deleted, bytes) = cleanup_orphans(state).await?;
    if !expired.is_empty() || clients > 0 || reports > 0 || deleted > 0 {
        tracing::info!(
            expired = expired.len(),
            client_hashes = clients,
            reports,
            orphan_blobs = deleted,
            orphan_bytes = bytes,
            "maintenance pass"
        );
    }
    Ok(())
}

async fn cleanup_orphans(state: &AppState) -> Result<(u64, i64), DbError> {
    let hashes = weft_db::orphan_hashes(&state.pool).await?;
    let mut deleted = 0_u64;
    let mut bytes = 0_i64;
    for hash in &hashes {
        if let Ok(blob_hash) = BlobHash::parse(hash) {
            if let Ok(Some(row)) = weft_db::get_blob_size(&state.pool, hash).await {
                bytes = bytes.saturating_add(row);
            }
            let _ = state.storage.delete(&blob_hash);
            let _ = state
                .storage
                .delete_thumbnail(&format!("thumbs/{hash}.jpg"));
            if let Err(error) = weft_db::delete_blob_row(&state.pool, hash).await {
                tracing::warn!(%error, hash, "could not delete orphan blob row");
                continue;
            }
            deleted += 1;
        }
    }
    Ok((deleted, bytes))
}

async fn admin_storage_cleanup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let (deleted, bytes) = cleanup_orphans(&state).await.map_err(ApiError::internal)?;
    Ok(Json(json!({ "deleted": deleted, "bytes": bytes })))
}

async fn admin_storage(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let report = weft_db::storage_report(&state.pool)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({
        "blobs": report.blobs,
        "bytes": report.bytes,
        "orphanBlobs": report.orphan_blobs,
        "orphanBytes": report.orphan_bytes,
    })))
}

async fn openapi() -> impl IntoResponse {
    (
        [("content-type", "application/json; charset=utf-8")],
        OPENAPI_JSON,
    )
}

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = state.config.server.admin_token.trim();
    if expected.is_empty() {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "The admin API is disabled; set server.admin_token.",
        ));
    }
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if authorization == expected {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "A valid admin bearer token is required.",
        ))
    }
}

fn summary(row: &ContentRow, state: &AppState) -> Value {
    let hash = row.hash.as_str();
    let base = state.config.server.external_url.trim_end_matches('/');
    json!({
        "hash": hash,
        "contentId": content_id(row).unwrap_or_default(),
        "mime": row.mime,
        "width": row.width,
        "height": row.height,
        "frames": row.frames,
        "durationMs": row.duration_ms,
        "title": row.title,
        "tags": row.tags,
        "licence": {
            "claim": row.licence_claim,
            "spdx": row.licence_spdx,
            "url": row.licence_url,
            "status": row.licence_status,
        },
        "mediaUrl": format!("{base}/media/{hash}"),
        "thumbnailUrl": row.thumbnail_key.as_ref().map(|_| format!("{base}/media/{hash}/thumbnail")),
        "manifestUrl": format!("{base}/api/v1/media/{hash}"),
        "source": row.source_url,
        "createdAt": row.created_at.to_rfc3339(),
        "status": row.status,
    })
}

fn admin_summary(row: &ContentRow, state: &AppState) -> Value {
    let mut value = summary(row, state);
    value["description"] = Value::String(row.description.clone());
    value["origin"] = Value::String(row.origin_node.clone());
    value["updatedAt"] = Value::String(row.updated_at.to_rfc3339());
    value
}

fn manifest_json(state: &AppState, row: &ContentRow) -> Result<Value, ApiError> {
    let source = source_value(state, row);
    let base = state.config.server.external_url.trim_end_matches('/');
    let mut value = json!({
        "version": "weft-manifest/0.1",
        "contentId": content_id(row)?,
        "hash": row.hash,
        "mime": row.mime,
        "size": row.size,
        "width": row.width,
        "height": row.height,
        "frames": row.frames,
        "durationMs": row.duration_ms,
        "createdAt": row.created_at.to_rfc3339(),
        "updatedAt": row.updated_at.to_rfc3339(),
        "origin": row.origin_node,
        "source": row.source_url,
        "thumbnailUrl": row.thumbnail_key.as_ref().map(|_| format!("{base}/media/{}/thumbnail", row.hash)),
        "sources": [source],
        "licence": {
            "claim": row.licence_claim,
            "spdx": row.licence_spdx,
            "url": row.licence_url,
            "status": row.licence_status,
            "evidence": row.evidence,
        },
        "moderation": {
            "state": row.status,
            "updatedAt": row.updated_at.to_rfc3339(),
        },
        "tags": row.tags,
    });
    let signature = state
        .signer
        .sign_value(&value)
        .map_err(ApiError::internal)?;
    value["signature"] = serde_json::to_value(signature).map_err(ApiError::internal)?;
    Ok(value)
}

fn source_value(state: &AppState, row: &ContentRow) -> Value {
    let base = state.config.server.external_url.trim_end_matches('/');
    json!({
        "nodeId": state.config.node.actor_url(),
        "url": format!("{base}/media/{}", row.hash),
        "verifiedAt": row.updated_at.to_rfc3339(),
        "bytes": row.size,
    })
}

fn content_id(row: &ContentRow) -> Result<String, ApiError> {
    Ok(ContentId::from_hash(BlobHash::parse(&row.hash).map_err(ApiError::internal)?).to_string())
}

fn parse_content_id(value: &str) -> Result<String, ApiError> {
    ContentId::parse(value)
        .map(|id| id.hash().to_string())
        .map_err(|_| ApiError::bad("invalid_content_id", "Expected urn:weft:sha256:<hash>."))
}

fn split_tags(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_ascii_lowercase())
        .collect()
}

fn normalize_tags(value: &str, max_tags: usize, max_chars: usize) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in value.split(',') {
        let tag = tag.trim().to_ascii_lowercase();
        if tag.is_empty() || tag.chars().count() > max_chars {
            continue;
        }
        if !tags.contains(&tag) {
            tags.push(tag);
        }
        if tags.len() >= max_tags {
            break;
        }
    }
    tags
}

fn encode_cursor(offset: i64) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(offset.to_string())
}

fn decode_cursor(cursor: Option<&str>) -> Result<i64, ApiError> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| ApiError::bad("invalid_cursor", "Invalid cursor."))?;
    let text =
        String::from_utf8(bytes).map_err(|_| ApiError::bad("invalid_cursor", "Invalid cursor."))?;
    text.parse::<i64>()
        .map_err(|_| ApiError::bad("invalid_cursor", "Invalid cursor."))
        .map(|offset| offset.max(0))
}

async fn request_logging(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let started = Instant::now();

    let mut response = next.run(request).await;
    state.metrics.record(response.status().as_u16());
    let header = HeaderValue::from_str(&request_id)
        .unwrap_or_else(|_| HeaderValue::from_static("invalid-request-id"));
    response.headers_mut().insert("x-request-id", header);

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis(),
        "request"
    );
    response
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    retry_after: Option<u64>,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            retry_after: None,
        }
    }

    fn bad(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    fn quota(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        retry_after: u64,
    ) -> Self {
        Self {
            retry_after: Some(retry_after),
            ..Self::new(status, code, message)
        }
    }

    fn with_retry(mut self, retry_after: u64) -> Self {
        self.retry_after = Some(retry_after);
        self
    }

    fn rate_limited(retry: u64) -> Self {
        Self {
            retry_after: Some(retry),
            ..Self::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "Too many uploads; retry later.",
            )
        }
    }

    fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!(%error, "request failed");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "The request could not be completed.",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            Json(json!({
                "error": {
                    "code": self.code,
                    "message": self.message,
                }
            })),
        )
            .into_response();
        if let Some(retry) = self.retry_after {
            if let Ok(value) = HeaderValue::from_str(&retry.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
        }
        response
    }
}

impl From<DbError> for ApiError {
    fn from(error: DbError) -> Self {
        Self::internal(error)
    }
}

impl From<StorageError> for ApiError {
    fn from(error: StorageError) -> Self {
        Self::internal(error)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self::internal(error)
    }
}

impl From<weft_core::signing::SigningError> for ApiError {
    fn from(error: weft_core::signing::SigningError) -> Self {
        Self::internal(error)
    }
}

impl From<DecodeWorkerError> for ApiError {
    fn from(error: DecodeWorkerError) -> Self {
        Self::internal(error)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        Self::internal(error)
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Signing(#[from] weft_core::signing::SigningError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod quota_tests {
    use axum::http::StatusCode;

    use super::enforce_quotas;

    fn config() -> weft_core::config::UploadConfig {
        weft_core::config::UploadConfig::default()
    }

    #[test]
    fn a_full_global_queue_is_rejected() {
        let error = enforce_quotas(&config(), 500, 0, 0, 1_000).unwrap_err();
        assert_eq!(error.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code, "quarantine_full");
    }

    #[test]
    fn a_client_pending_quota_is_rejected() {
        let error = enforce_quotas(&config(), 0, 10, 0, 1_000).unwrap_err();
        assert_eq!(error.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(error.code, "pending_quota");
    }

    #[test]
    fn the_daily_byte_budget_is_inclusive() {
        assert!(enforce_quotas(&config(), 0, 0, 100_000_000 - 100, 100).is_ok());
        let error = enforce_quotas(&config(), 0, 0, 100_000_000 - 99, 100).unwrap_err();
        assert_eq!(error.code, "daily_quota");
    }
}
