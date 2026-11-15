// SPDX-License-Identifier: AGPL-3.0-or-later

//! PostgreSQL connection, migrations and content queries.

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{FromRow, Postgres, QueryBuilder};
use thiserror::Error;
use uuid::Uuid;
use weft_core::config::{DatabaseConfig, NodeConfig};

const CONTENT_SELECT: &str = r#"
SELECT c.id, c.hash, c.title, c.description, c.status,
       b.mime, b.size, b.width, b.height, b.frames, b.duration_ms,
       b.storage_key, b.thumbnail_key,
       c.licence_spdx, c.licence_url, c.licence_claim, c.licence_status,
       c.source_url, c.views,
       c.published_at, c.created_at, c.updated_at, c.origin_node,
       COALESCE(array_agg(t.tag ORDER BY t.tag) FILTER (WHERE t.tag IS NOT NULL), '{}') AS tags,
       COALESCE((SELECT array_agg(le.url ORDER BY le.created_at)
                 FROM licence_evidence le WHERE le.content_id = c.id), '{}') AS evidence
FROM content c
JOIN blobs b ON b.hash = c.hash
LEFT JOIN content_tags t ON t.content_id = c.id
"#;

#[derive(Debug, Clone, FromRow)]
pub struct ContentRow {
    pub id: Uuid,
    pub hash: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub mime: String,
    pub size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub frames: Option<i32>,
    pub duration_ms: Option<i32>,
    pub storage_key: String,
    pub thumbnail_key: Option<String>,
    pub licence_spdx: Option<String>,
    pub licence_url: Option<String>,
    pub licence_claim: Option<String>,
    pub licence_status: String,
    pub source_url: Option<String>,
    pub views: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub origin_node: String,
    pub tags: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct AdminStats {
    pub active: i64,
    pub pending: i64,
    pub removed: i64,
    pub bytes: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct ReportRow {
    pub id: Uuid,
    pub hash: Option<String>,
    pub reason: String,
    pub details: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ContentQuery {
    pub text: String,
    pub tags: Vec<String>,
    pub blocked_tags: Vec<String>,
    pub sort: String,
    pub limit: i64,
    pub offset: i64,
}

impl Default for ContentQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            tags: Vec::new(),
            blocked_tags: Vec::new(),
            sort: "newest".to_string(),
            limit: 20,
            offset: 0,
        }
    }
}

/// A search backend abstraction. The PostgreSQL implementation is the default;
/// a future Meilisearch adapter can implement the same contract.
#[async_trait]
pub trait SearchIndex: Send + Sync {
    async fn search(&self, query: &ContentQuery) -> Result<Vec<ContentRow>, DbError>;
}

/// PostgreSQL-backed search.
#[derive(Debug, Clone)]
pub struct PostgresSearch {
    pool: PgPool,
}

impl PostgresSearch {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SearchIndex for PostgresSearch {
    async fn search(&self, query: &ContentQuery) -> Result<Vec<ContentRow>, DbError> {
        list_content(&self.pool, query).await
    }
}

/// Connect using the validated configuration.
pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, DbError> {
    connect_url(
        &config.url,
        config.max_connections,
        Duration::from_secs(config.connect_timeout_seconds),
    )
    .await
}

/// Connect with an explicit URL, used by tests and operators.
pub async fn connect_url(
    url: &str,
    max_connections: u32,
    connect_timeout: Duration,
) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(connect_timeout)
        .connect(url)
        .await?;
    Ok(pool)
}

/// Apply the forward-only SQL migrations embedded in the binary.
pub async fn migrate(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    tracing::info!("database migrations are current");
    Ok(())
}

/// Check that PostgreSQL answers a trivial query.
pub async fn health(pool: &PgPool) -> Result<(), DbError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

/// Insert or update the local node row.
pub async fn ensure_node(pool: &PgPool, node: &NodeConfig) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO node (actor_url, public_url, name, description)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (actor_url) DO UPDATE
        SET public_url = EXCLUDED.public_url,
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            updated_at = now()
        "#,
    )
    .bind(node.actor_url())
    .bind(&node.public_url)
    .bind(&node.name)
    .bind(&node.description)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch one content record by hash. `any_status` includes quarantined/removed rows.
pub async fn get_content(
    pool: &PgPool,
    hash: &str,
    any_status: bool,
) -> Result<Option<ContentRow>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new(CONTENT_SELECT);
    query.push(" WHERE c.hash = ").push_bind(hash);
    if !any_status {
        query.push(" AND c.status = 'active'");
    }
    query.push(" GROUP BY c.id, b.hash LIMIT 1");
    Ok(query
        .build_query_as::<ContentRow>()
        .fetch_optional(pool)
        .await?)
}

/// List active content with text, tag, sort and offset pagination.
pub async fn list_content(
    pool: &PgPool,
    filter: &ContentQuery,
) -> Result<Vec<ContentRow>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new(CONTENT_SELECT);
    query.push(" WHERE c.status = 'active'");

    let text = filter.text.trim();
    if !text.is_empty() {
        let pattern = format!("%{text}%");
        query
            .push(" AND (c.title ILIKE ")
            .push_bind(pattern.clone())
            .push(" OR c.description ILIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if !filter.tags.is_empty() {
        query
            .push(" AND EXISTS (SELECT 1 FROM content_tags ft WHERE ft.content_id = c.id AND ft.tag = ANY(")
            .push_bind(filter.tags.clone())
            .push("))");
    }
    if !filter.blocked_tags.is_empty() {
        query
            .push(" AND NOT EXISTS (SELECT 1 FROM content_tags bt WHERE bt.content_id = c.id AND bt.tag = ANY(")
            .push_bind(filter.blocked_tags.clone())
            .push("))");
    }

    query.push(" GROUP BY c.id, b.hash");
    if filter.sort == "relevance" && !text.is_empty() {
        query
            .push(" ORDER BY ts_rank(c.search, websearch_to_tsquery('simple', ")
            .push_bind(text.to_string())
            .push(")) DESC, c.published_at DESC NULLS LAST, c.id DESC");
    } else if filter.sort == "trending" {
        query.push(" ORDER BY c.views DESC, c.published_at DESC NULLS LAST, c.id DESC");
    } else {
        query.push(" ORDER BY c.published_at DESC NULLS LAST, c.id DESC");
    }
    query
        .push(" LIMIT ")
        .push_bind(filter.limit.clamp(1, 100))
        .push(" OFFSET ")
        .push_bind(filter.offset.max(0));

    Ok(query.build_query_as::<ContentRow>().fetch_all(pool).await?)
}

/// Pending uploads for the review queue.
pub async fn review_queue(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<ContentRow>, DbError> {
    let mut query = QueryBuilder::<Postgres>::new(CONTENT_SELECT);
    query
        .push(" WHERE c.status = 'pending' GROUP BY c.id, b.hash ORDER BY c.created_at ASC LIMIT ")
        .push_bind(limit.clamp(1, 100))
        .push(" OFFSET ")
        .push_bind(offset.max(0));
    Ok(query.build_query_as::<ContentRow>().fetch_all(pool).await?)
}

/// Popular tags among active content.
pub async fn tags(pool: &PgPool) -> Result<Vec<TagCount>, DbError> {
    Ok(sqlx::query_as::<_, TagCount>(
        r#"
        SELECT t.tag, COUNT(*)::bigint AS count
        FROM content_tags t
        JOIN content c ON c.id = t.content_id
        WHERE c.status = 'active'
        GROUP BY t.tag
        ORDER BY count DESC, t.tag ASC
        LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await?)
}

/// Insert a validated upload, its blob and tags in one transaction.
pub struct NewUpload<'a> {
    pub hash: &'a str,
    pub mime: &'a str,
    pub size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub frames: Option<i32>,
    pub duration_ms: Option<i32>,
    pub storage_key: &'a str,
    pub thumbnail_key: Option<&'a str>,
    pub title: &'a str,
    pub description: &'a str,
    pub origin_node: &'a str,
    pub status: &'a str,
    pub licence_spdx: Option<&'a str>,
    pub licence_url: Option<&'a str>,
    pub licence_claim: Option<&'a str>,
    pub licence_status: &'a str,
    pub source_url: Option<&'a str>,
    pub licence_evidence: &'a [String],
    pub tags: &'a [String],
    pub review: &'a str,
    pub client_hash: Option<&'a [u8]>,
}

#[derive(Debug, Clone)]
pub struct RecordedUpload {
    pub content_id: Uuid,
    pub upload_id: Uuid,
    pub content_status: String,
}

pub async fn record_upload(
    pool: &PgPool,
    upload: &NewUpload<'_>,
) -> Result<RecordedUpload, DbError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO blobs
            (hash, size, mime, width, height, frames, duration_ms, storage_key, thumbnail_key, verified_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
        ON CONFLICT (hash) DO UPDATE SET verified_at = now()
        "#,
    )
    .bind(upload.hash)
    .bind(upload.size)
    .bind(upload.mime)
    .bind(upload.width)
    .bind(upload.height)
    .bind(upload.frames)
    .bind(upload.duration_ms)
    .bind(upload.storage_key)
    .bind(upload.thumbnail_key)
    .execute(&mut *tx)
    .await?;

    let (content_id, content_status): (Uuid, String) = sqlx::query_as(
        r#"
        INSERT INTO content
            (hash, title, description, origin_node, status, licence_spdx, licence_url,
             licence_claim, licence_status, source_url, published_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                CASE WHEN $5 = 'active' THEN now() ELSE NULL END)
        ON CONFLICT (hash, origin_node) DO UPDATE
        SET updated_at = now(),
            source_url = COALESCE(EXCLUDED.source_url, content.source_url)
        RETURNING id, status
        "#,
    )
    .bind(upload.hash)
    .bind(upload.title)
    .bind(upload.description)
    .bind(upload.origin_node)
    .bind(upload.status)
    .bind(upload.licence_spdx)
    .bind(upload.licence_url)
    .bind(upload.licence_claim)
    .bind(upload.licence_status)
    .bind(upload.source_url)
    .fetch_one(&mut *tx)
    .await?;

    for tag in upload.tags {
        sqlx::query(
            "INSERT INTO content_tags (content_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(content_id)
        .bind(tag)
        .execute(&mut *tx)
        .await?;
    }

    for url in upload.licence_evidence {
        sqlx::query("INSERT INTO licence_evidence (content_id, url) VALUES ($1, $2)")
            .bind(content_id)
            .bind(url)
            .execute(&mut *tx)
            .await?;
    }

    let upload_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO uploads (hash, status, review, client_hash)
        VALUES ($1, 'stored', $2, $3)
        RETURNING id
        "#,
    )
    .bind(upload.hash)
    .bind(upload.review)
    .bind(upload.client_hash)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(RecordedUpload {
        content_id,
        upload_id,
        content_status,
    })
}

/// Change a content status and append a local moderation event.
pub async fn set_status(
    pool: &PgPool,
    hash: &str,
    status: &str,
    reason: &str,
    actor: &str,
) -> Result<Option<ContentRow>, DbError> {
    let category = match status {
        "active" => "CONTENT_REINSTATED",
        "restricted" => "CONTENT_RESTRICTED",
        "removed" | "rejected" => "CONTENT_REMOVED",
        _ => "CONTENT_REPORTED",
    };
    let mut tx = pool.begin().await?;

    let content_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE content
        SET status = $2,
            updated_at = now(),
            published_at = CASE WHEN $2 = 'active' THEN COALESCE(published_at, now()) ELSE published_at END,
            removed_at = CASE WHEN $2 IN ('removed', 'rejected') THEN now() ELSE NULL END,
            tombstone_reason = CASE WHEN $2 IN ('removed', 'rejected') THEN $3 ELSE NULL END
        WHERE hash = $1
        RETURNING id
        "#,
    )
    .bind(hash)
    .bind(status)
    .bind(reason)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(content_id) = content_id {
        sqlx::query(
            r#"
            INSERT INTO moderation_events
                (content_id, hash, category, scope, issuer, reason)
            VALUES ($1, $2, $3, 'local', $4, $5)
            "#,
        )
        .bind(content_id)
        .bind(hash)
        .bind(category)
        .bind(actor)
        .bind(reason)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO audit_log (actor, action, target, detail) VALUES ($1, $2, $3, $4)",
        )
        .bind(actor)
        .bind(format!("content.{status}"))
        .bind(hash)
        .bind(serde_json::json!({ "reason": reason }))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    get_content(pool, hash, true).await
}

/// Record a content report and its local moderation event.
pub async fn insert_report(
    pool: &PgPool,
    hash: &str,
    reason: &str,
    details: &str,
    contact_hash: Option<&[u8]>,
) -> Result<Uuid, DbError> {
    let mut tx = pool.begin().await?;
    let content_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM content WHERE hash = $1")
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await?;
    let report_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO reports (content_id, hash, reason, details, contact_hash)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(content_id)
    .bind(hash)
    .bind(reason)
    .bind(details)
    .bind(contact_hash)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO moderation_events
            (content_id, hash, category, scope, issuer, reason, payload)
        VALUES ($1, $2, 'CONTENT_REPORTED', 'local', 'local', $3, $4)
        "#,
    )
    .bind(content_id)
    .bind(hash)
    .bind(reason)
    .bind(serde_json::json!({ "reportId": report_id.to_string() }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(report_id)
}

/// Record a licence dispute, mark the local licence status disputed, and report it.
pub async fn insert_licence_dispute(
    pool: &PgPool,
    hash: &str,
    claim: &str,
    evidence: &[String],
    contact_hash: Option<&[u8]>,
) -> Result<Uuid, DbError> {
    let mut tx = pool.begin().await?;
    let content_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM content WHERE hash = $1")
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await?;
    let details = serde_json::json!({ "claim": claim, "evidence": evidence });
    let report_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO reports (content_id, hash, reason, details, contact_hash)
        VALUES ($1, $2, 'licence', $3, $4)
        RETURNING id
        "#,
    )
    .bind(content_id)
    .bind(hash)
    .bind(details.to_string())
    .bind(contact_hash)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE content SET licence_status = 'disputed', updated_at = now() WHERE hash = $1",
    )
    .bind(hash)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO moderation_events
            (content_id, hash, category, scope, issuer, reason, payload)
        VALUES ($1, $2, 'LICENCE_DISPUTED', 'local', 'local', 'licence_dispute', $3)
        "#,
    )
    .bind(content_id)
    .bind(hash)
    .bind(details)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(report_id)
}

/// Update the local licence status and record the admin decision.
pub async fn set_licence_status(
    pool: &PgPool,
    hash: &str,
    status: &str,
    note: &str,
    actor: &str,
) -> Result<Option<ContentRow>, DbError> {
    let mut tx = pool.begin().await?;
    let content_id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE content SET licence_status = $2, updated_at = now() WHERE hash = $1 RETURNING id",
    )
    .bind(hash)
    .bind(status)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(content_id) = content_id {
        sqlx::query(
            "INSERT INTO audit_log (actor, action, target, detail) VALUES ($1, $2, $3, $4)",
        )
        .bind(actor)
        .bind("content.licence")
        .bind(hash)
        .bind(serde_json::json!({ "status": status, "note": note, "contentId": content_id.to_string() }))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    get_content(pool, hash, true).await
}

/// Recent reports for the admin surface.
pub async fn list_reports(pool: &PgPool, limit: i64) -> Result<Vec<ReportRow>, DbError> {
    Ok(sqlx::query_as::<_, ReportRow>(
        r#"
        SELECT id, hash, reason, details, status, created_at
        FROM reports
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?)
}

/// Counts for the admin stats endpoint.
pub async fn admin_stats(pool: &PgPool) -> Result<AdminStats, DbError> {
    Ok(sqlx::query_as::<_, AdminStats>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE c.status = 'active')::bigint AS active,
            COUNT(*) FILTER (WHERE c.status = 'pending')::bigint AS pending,
            COUNT(*) FILTER (WHERE c.status = 'removed')::bigint AS removed,
            COALESCE(SUM(b.size) FILTER (WHERE c.status = 'active'), 0)::bigint AS bytes
        FROM content c
        LEFT JOIN blobs b ON b.hash = c.hash
        "#,
    )
    .fetch_one(pool)
    .await?)
}

/// Delete a blob only when no active, restricted or pending content references it.
pub async fn delete_blob_if_orphan(pool: &PgPool, hash: &str) -> Result<bool, DbError> {
    let references: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM content
        WHERE hash = $1 AND status IN ('active', 'restricted', 'pending')
        "#,
    )
    .bind(hash)
    .fetch_one(pool)
    .await?;
    if references > 0 {
        return Ok(false);
    }
    sqlx::query("DELETE FROM blobs WHERE hash = $1")
        .bind(hash)
        .execute(pool)
        .await?;
    Ok(true)
}

/// Increment the local view counter for an active content row.
pub async fn increment_views(pool: &PgPool, hash: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE content SET views = views + 1 WHERE hash = $1 AND status = 'active'")
        .bind(hash)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow)]
pub struct BlockRule {
    pub id: Uuid,
    pub kind: String,
    pub value: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

pub async fn block_rules(pool: &PgPool) -> Result<Vec<BlockRule>, DbError> {
    Ok(sqlx::query_as::<_, BlockRule>(
        "SELECT id, kind, value, reason, created_at FROM node_blocks ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn add_block(
    pool: &PgPool,
    kind: &str,
    value: &str,
    reason: &str,
) -> Result<Uuid, DbError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO node_blocks (kind, value, reason) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(kind)
    .bind(value)
    .bind(reason)
    .fetch_one(pool)
    .await?)
}

pub async fn delete_block(pool: &PgPool, id: Uuid) -> Result<bool, DbError> {
    let result = sqlx::query("DELETE FROM node_blocks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Number of items waiting for review.
pub async fn pending_count(pool: &PgPool) -> Result<i64, DbError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM content WHERE status = 'pending'")
            .fetch_one(pool)
            .await?,
    )
}

/// Distinct pending items uploaded by one client.
pub async fn pending_count_for_client(pool: &PgPool, client_hash: &[u8]) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT u.hash)::bigint
        FROM uploads u
        JOIN content c ON c.hash = u.hash
        WHERE u.client_hash = $1
          AND u.review = 'queued'
          AND c.status = 'pending'
        "#,
    )
    .bind(client_hash)
    .fetch_one(pool)
    .await?)
}

/// Uploaded bytes accepted from one client in the last `hours` hours.
pub async fn uploaded_bytes_since(
    pool: &PgPool,
    client_hash: &[u8],
    hours: i64,
) -> Result<i64, DbError> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(b.size), 0)::bigint
        FROM uploads u
        JOIN blobs b ON b.hash = u.hash
        WHERE u.client_hash = $1
          AND u.status = 'stored'
          AND u.created_at > now() - ($2::int * interval '1 hour')
        "#,
    )
    .bind(client_hash)
    .bind(hours as i32)
    .fetch_one(pool)
    .await?)
}

/// Delete pending content older than `days`; returns the removed hashes.
pub async fn expire_pending(pool: &PgPool, days: u32) -> Result<Vec<String>, DbError> {
    Ok(sqlx::query_scalar(
        r#"
        DELETE FROM content
        WHERE status = 'pending'
          AND created_at < now() - ($1::int * interval '1 day')
        RETURNING hash
        "#,
    )
    .bind(days as i32)
    .fetch_all(pool)
    .await?)
}

/// Clear old client pseudonyms and delete expired reports.
pub async fn retention_cleanup(
    pool: &PgPool,
    client_hash_days: u32,
    report_days: u32,
) -> Result<(u64, u64), DbError> {
    let clients = sqlx::query(
        r#"
        UPDATE uploads
        SET client_hash = NULL
        WHERE client_hash IS NOT NULL
          AND created_at < now() - ($1::int * interval '1 day')
        "#,
    )
    .bind(client_hash_days as i32)
    .execute(pool)
    .await?
    .rows_affected();
    let reports =
        sqlx::query("DELETE FROM reports WHERE created_at < now() - ($1::int * interval '1 day')")
            .bind(report_days as i32)
            .execute(pool)
            .await?
            .rows_affected();
    Ok((clients, reports))
}

/// Hashes with a blob but no content reference.
pub async fn orphan_hashes(pool: &PgPool) -> Result<Vec<String>, DbError> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT b.hash FROM blobs b
        WHERE NOT EXISTS (SELECT 1 FROM content c WHERE c.hash = b.hash)
        ORDER BY b.hash
        "#,
    )
    .fetch_all(pool)
    .await?)
}

/// Size of a blob row, used by the orphan cleanup report.
pub async fn get_blob_size(pool: &PgPool, hash: &str) -> Result<Option<i64>, DbError> {
    Ok(sqlx::query_scalar("SELECT size FROM blobs WHERE hash = $1")
        .bind(hash)
        .fetch_optional(pool)
        .await?)
}

/// Delete a blob row after its bytes have been removed.
pub async fn delete_blob_row(pool: &PgPool, hash: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM blobs WHERE hash = $1")
        .bind(hash)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow)]
pub struct StorageReport {
    pub blobs: i64,
    pub bytes: i64,
    pub orphan_blobs: i64,
    pub orphan_bytes: i64,
}

pub async fn storage_report(pool: &PgPool) -> Result<StorageReport, DbError> {
    Ok(sqlx::query_as::<_, StorageReport>(
        r#"
        SELECT
            COUNT(*)::bigint AS blobs,
            COALESCE(SUM(b.size), 0)::bigint AS bytes,
            COUNT(*) FILTER (WHERE NOT EXISTS (
                SELECT 1 FROM content c WHERE c.hash = b.hash
            ))::bigint AS orphan_blobs,
            COALESCE(SUM(b.size) FILTER (WHERE NOT EXISTS (
                SELECT 1 FROM content c WHERE c.hash = b.hash
            )), 0)::bigint AS orphan_bytes
        FROM blobs b
        "#,
    )
    .fetch_one(pool)
    .await?)
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}
