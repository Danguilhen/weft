// SPDX-License-Identifier: AGPL-3.0-or-later

//! PostgreSQL connection, migrations and the local node row.

use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};
use thiserror::Error;
use weft_core::config::{DatabaseConfig, NodeConfig};

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

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}
