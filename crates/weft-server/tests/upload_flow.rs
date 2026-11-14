// SPDX-License-Identifier: AGPL-3.0-or-later

//! End-to-end upload → review → search → media smoke test.
//!
//! The decode worker is replaced by a tiny shell script so this test exercises
//! the server, database, storage and admin flow without depending on image
//! decoders. The real worker is tested in `weft-validation`.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
use weft_core::{AppConfig, NodeSigner};
use weft_server::metrics::Metrics;
use weft_server::{build_router, AppState, NodePublic};
use weft_storage::LocalStore;

#[tokio::test]
#[ignore = "requires WEFT_TEST_DATABASE_URL"]
async fn upload_review_search_and_fetch() {
    let url = std::env::var("WEFT_TEST_DATABASE_URL")
        .expect("set WEFT_TEST_DATABASE_URL to a disposable PostgreSQL database");
    let pool = weft_db::connect_url(&url, 2, Duration::from_secs(5))
        .await
        .expect("connect to PostgreSQL");
    weft_db::migrate(&pool).await.expect("apply migrations");

    let directory = tempfile::tempdir().expect("temp directory");
    let worker = directory.path().join("weft-decode");
    write_fake_worker(&worker);

    let mut config = AppConfig::default();
    config.server.admin_token = "test-admin".to_string();
    config.server.external_url = "http://127.0.0.1:8080".to_string();
    config.storage.path = directory.path().join("data").to_string_lossy().to_string();
    config.decode.worker_path = worker.to_string_lossy().to_string();
    let signer = NodeSigner::load_or_create(
        directory.path().join("node.key"),
        format!("{}#key-1", config.node.actor_url()),
    )
    .expect("node key");
    let node = NodePublic::from_config(&config);
    let storage = Arc::new(LocalStore::new(&config.storage.path).expect("storage"));
    let state = AppState::new(
        pool,
        storage,
        Arc::new(Metrics::default()),
        node,
        config,
        signer,
    );
    let app = build_router(state);

    let gif = b"GIF89a-fake-upload";
    let upload = multipart_body(gif, "Test GIF", "cat,test");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upload")
                .header("x-forwarded-for", "127.0.0.1")
                .header(
                    header::CONTENT_TYPE,
                    "multipart/form-data; boundary=weft-boundary",
                )
                .body(Body::from(upload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    let uploaded: Value = serde_json::from_slice(&body).unwrap();
    let hash = uploaded["hash"].as_str().unwrap().to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/queue")
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/content/{hash}/approve"))
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=Test&tags=cat")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains(&hash));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/media/{hash}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/gif"
    );
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_eq!(&body[..], gif);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/media/{hash}"))
                .header(header::RANGE, "bytes=0-5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        response.headers().get(header::CONTENT_RANGE).unwrap(),
        format!("bytes 0-5/{}", gif.len()).as_str()
    );

    let content_id = uploaded["contentId"].as_str().unwrap();
    let report = serde_json::json!({
        "contentId": content_id,
        "reason": "licence",
        "details": "Test report"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/report")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(report.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/content/{hash}/restrict"))
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/media/{hash}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/content/{hash}/reinstate"))
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/reports")
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/content/{hash}/remove"))
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/media/{hash}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/stats")
                .header(header::AUTHORIZATION, "Bearer test-admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

fn multipart_body(file: &[u8], title: &str, tags: &str) -> Vec<u8> {
    let boundary = "weft-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.gif\"\r\nContent-Type: image/gif\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(
        format!(
            "\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\n{title}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"tags\"\r\n\r\n{tags}\r\n--{boundary}--\r\n"
        )
        .as_bytes(),
    );
    body
}

fn write_fake_worker(path: &Path) {
    let script = r#"#!/bin/sh
thumbnail=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --thumbnail) thumbnail="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf 'fake-thumbnail' > "$thumbnail"
printf '{"mime":"image/gif","width":16,"height":16,"frames":1,"duration_ms":0}'
"#;
    std::fs::write(path, script).unwrap();
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}
