# Weft — architecture decision records

ADRs record decisions that affect the protocol, security, storage, licensing or
operations of the project. Small factual corrections are allowed; a changed
decision is recorded in a new ADR that supersedes the old one.

## Index

| ADR | Title | Status |
|---|---|---|
| [ADR-001](ADR-001-two-layer-architecture.md) | ActivityPub metadata plus a media distribution protocol | Accepted |
| [ADR-002](ADR-002-rust-stack.md) | Rust, Axum, SQLx, PostgreSQL, Postgres-first search | Accepted |
| [ADR-003](ADR-003-content-addressing.md) | SHA-256 content identity and metadata separation | Accepted |
| [ADR-004](ADR-004-pull-replication.md) | Pull-based replication and signed receipts | Accepted |
| [ADR-005](ADR-005-anonymous-uploads.md) | Anonymous uploads with a configurable review queue | Accepted |
| [ADR-006](ADR-006-software-licensing.md) | AGPL server and MIT core/client | Accepted |
| [ADR-007](ADR-007-xep-0363-delivery.md) | XMPP clients send through XEP-0363 | Accepted |
| [ADR-008](ADR-008-activitypub-metadata-only.md) | ActivityPub announces metadata, never media bytes | Accepted |

## Creating an ADR

1. Copy [`TEMPLATE.md`](TEMPLATE.md) to `ADR-<next>-<slug>.md`.
2. Fill in context, decision, consequences and alternatives.
3. Add it to the index with status `Proposed`.
4. After review, change the status to `Accepted`.
5. If it replaces an old decision, mark the old one `Superseded by ADR-XXX` and
   link the new ADR.
