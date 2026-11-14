// SPDX-License-Identifier: AGPL-3.0-or-later

//! Weft HTTP server for Milestone 0.

pub mod metrics;

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use weft_core::config::{AppConfig, LoggingConfig};
use weft_db::DbError;
use weft_storage::{BlobStore, LocalStore, StorageError};

use crate::metrics::Metrics;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub storage: Arc<dyn BlobStore>,
    pub metrics: Arc<Metrics>,
    pub node: Arc<NodePublic>,
    pub max_json_bytes: usize,
    pub metrics_enabled: bool,
}

impl AppState {
    pub fn new(
        pool: sqlx::PgPool,
        storage: Arc<dyn BlobStore>,
        metrics: Arc<Metrics>,
        node: NodePublic,
        max_json_bytes: usize,
        metrics_enabled: bool,
    ) -> Self {
        Self {
            pool,
            storage,
            metrics,
            node: Arc::new(node),
            max_json_bytes,
            metrics_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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
    let max_json_bytes = state.max_json_bytes;
    let middleware_state = state.clone();

    let mut router = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/v1/node", get(node));

    if metrics_enabled {
        router = router.route("/metrics", get(metrics));
    }

    router
        .layer(middleware::from_fn_with_state(
            middleware_state,
            request_logging,
        ))
        .layer(DefaultBodyLimit::max(max_json_bytes))
        .with_state(state)
}

/// Run the node until Ctrl-C.
pub async fn run(config: AppConfig) -> Result<(), ServerError> {
    init_logging(&config.logging);
    tracing::info!(bind = %config.server.bind, "starting Weft");

    let pool = weft_db::connect(&config.database).await?;
    weft_db::migrate(&pool).await?;
    weft_db::ensure_node(&pool, &config.node).await?;

    let storage = Arc::new(LocalStore::new(&config.storage.path)?);
    let state = AppState::new(
        pool,
        storage,
        Arc::new(Metrics::default()),
        NodePublic::from_config(&config),
        config.server.max_json_bytes,
        config.metrics.enabled,
    );
    let app = build_router(state);

    let listener = TcpListener::bind(&config.server.bind).await?;
    tracing::info!(bind = %config.server.bind, "listening");
    axum::serve(listener, app)
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

#[derive(Serialize)]
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

#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
