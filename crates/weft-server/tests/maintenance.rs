// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retention and quarantine-expiry checks against PostgreSQL.

use std::time::Duration;

#[tokio::test]
#[ignore = "requires WEFT_TEST_DATABASE_URL"]
async fn quarantine_and_retention_cleanup_are_bounded() {
    let url = std::env::var("WEFT_TEST_DATABASE_URL")
        .expect("set WEFT_TEST_DATABASE_URL to a disposable PostgreSQL database");
    let pool = weft_db::connect_url(&url, 2, Duration::from_secs(5))
        .await
        .expect("connect to PostgreSQL");
    weft_db::migrate(&pool).await.expect("apply migrations");

    let hash = weft_core::hash_bytes(b"weft-maintenance-test").to_string();
    sqlx::query(
        "INSERT INTO blobs (hash, size, mime, storage_key, verified_at) VALUES ($1, 32, 'image/gif', $1, now()) ON CONFLICT (hash) DO NOTHING",
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO content (hash, title, origin_node, status, created_at)
        VALUES ($1, 'Old pending', 'https://example.test/actor', 'pending', now() - interval '10 days')
        ON CONFLICT (hash, origin_node) DO UPDATE SET created_at = EXCLUDED.created_at, status = 'pending'
        "#,
    )
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();

    let expired = weft_db::expire_pending(&pool, 7).await.unwrap();
    assert!(expired.contains(&hash));
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM content WHERE hash = $1")
        .bind(&hash)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, 0);

    let old_upload: sqlx::types::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO uploads (hash, status, review, client_hash, created_at)
        VALUES ($1, 'stored', 'queued', $2, now() - interval '10 days')
        RETURNING id
        "#,
    )
    .bind(&hash)
    .bind(vec![7_u8; 16])
    .fetch_one(&pool)
    .await
    .unwrap();
    let old_report: sqlx::types::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO reports (hash, reason, contact_hash, created_at)
        VALUES ($1, 'spam', $2, now() - interval '100 days')
        RETURNING id
        "#,
    )
    .bind(&hash)
    .bind(vec![7_u8; 16])
    .fetch_one(&pool)
    .await
    .unwrap();

    let (clients, reports) = weft_db::retention_cleanup(&pool, 7, 90).await.unwrap();
    assert!(clients >= 1);
    assert!(reports >= 1);
    let client_hash: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT client_hash FROM uploads WHERE id = $1")
            .bind(old_upload)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(client_hash.is_none());
    let report_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports WHERE id = $1")
        .bind(old_report)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(report_count, 0);
}
