# ADR-002 — Rust, Axum, SQLx, PostgreSQL and Postgres-first search

- Status: Accepted
- Date: 2026-11-14
- Affects: storage, operations, API

## Context

Weft needs a small, secure, self-hostable server with strict upload validation,
predictable memory use and a typed client that Jabu can embed. The brief
suggested Go or Rust and Meilisearch; both are viable, but a first version should
minimise moving parts for a 2 vCPU / 2 GB node.

## Decision

Use Rust for the server, CLI and client:

- **Axum + Tokio** for HTTP.
- **SQLx + PostgreSQL** for persistence and migrations.
- **PostgreSQL full-text search** (`tsvector`, `pg_trgm`) for the MVP.
- **Local filesystem** storage by default; S3/MinIO optional.
- **Meilisearch** is a later optional service behind a `SearchIndex` trait, not a
  dependency of the default deployment.

The workspace is split so `weft-core` and `weft-client` are MIT and have no
server dependencies.

## Consequences

- Positive: one language for server and client; the Jabu client can reuse the
  typed API.
- Positive: the default deployment is app + PostgreSQL, which fits a small VPS.
- Positive: SQLx migrations and compile-time checked queries.
- Negative: PostgreSQL FTS is weaker than Meilisearch for fuzzy multilingual
  search; the trait keeps the option open.
- Negative: Rust compilation is slower for contributors; CI caches are needed.
- Follow-up: measure search quality on a real corpus before adding Meilisearch.

## Alternatives considered

| Alternative | Why rejected for the MVP |
|---|---|
| Go backend | Would duplicate the client language and lose the shared types |
| Meilisearch from day one | Extra service, index sync and operational burden |
| SQLite | Weak concurrent search and migration story for a federated node |
| Elasticsearch/OpenSearch | Too heavy for a small node |

## References

- [`../ARCHITECTURE.md`](../ARCHITECTURE.md)
- [`../DATA_MODEL.md`](../DATA_MODEL.md)
- [`../CONFIG.md`](../CONFIG.md)
