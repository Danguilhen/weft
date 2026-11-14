// SPDX-License-Identifier: AGPL-3.0-or-later

//! PostgreSQL-backed smoke test.
//!
//! Run with a disposable database:
//!
//! ```bash
//! WEFT_TEST_DATABASE_URL=postgres://weft:weft@127.0.0.1:5432/weft \
//!   cargo test -p weft-server --test postgres_smoke -- --ignored
//! ```

use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use weft_core::AppConfig;
use weft_core::NodeSigner;
use weft_server::metrics::Metrics;
use weft_server::{build_router, AppState, NodePublic};
use weft_storage::LocalStore;

#[tokio::test]
#[ignore = "requires WEFT_TEST_DATABASE_URL"]
async fn health_ready_metrics_and_node_descriptor() {
    let url = std::env::var("WEFT_TEST_DATABASE_URL")
        .expect("set WEFT_TEST_DATABASE_URL to a disposable PostgreSQL database");
    let pool = weft_db::connect_url(&url, 2, Duration::from_secs(5))
        .await
        .expect("connect to PostgreSQL");
    weft_db::migrate(&pool).await.expect("apply migrations");

    let directory = tempfile::tempdir().expect("temp directory");
    let storage = Arc::new(LocalStore::new(directory.path()).expect("local store"));
    let config = AppConfig::default();
    let signer = NodeSigner::load_or_create(
        directory.path().join("node.key"),
        format!("{}#key-1", config.node.actor_url()),
    )
    .expect("node key");
    let node = NodePublic::from_config(&config);
    let state = AppState::new(
        pool,
        storage,
        Arc::new(Metrics::default()),
        node,
        config,
        signer,
    );
    let app = build_router(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"ok");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("\"ready\""));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("weft_requests_total"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/node")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("weft-wmp/0.1"));
    assert!(text.contains("127.0.0.1"));
}
