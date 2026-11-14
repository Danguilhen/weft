# Weft — architecture decision records

ADRs record decisions that affect the protocol, security, storage, licensing or
operations of the project. Small factual corrections are allowed; a changed
decision is recorded in a new ADR that supersedes the old one.

## Index

| ADR | Title | Status |
|---|---|---|
| [ADR-001](ADR-001-two-layer-architecture.md) | ActivityPub metadata plus the Weft Media Protocol (WMP) | Accepted |
| [ADR-002](ADR-002-rust-stack.md) | Rust, Axum, SQLx, PostgreSQL, Postgres-first search | Accepted |
| [ADR-003](ADR-003-content-addressing.md) | SHA-256 content identity and metadata separation | Accepted |
| [ADR-004](ADR-004-pull-replication.md) | Pull-based replication and signed receipts | Accepted |
| [ADR-005](ADR-005-anonymous-uploads.md) | Anonymous uploads with a configurable review queue | Accepted |
| [ADR-006](ADR-006-software-licensing.md) | AGPL server and MIT core/client | Accepted |
| [ADR-007](ADR-007-xep-0363-delivery.md) | XMPP clients send through XEP-0363 | Accepted |
| [ADR-008](ADR-008-activitypub-metadata-only.md) | ActivityPub announces metadata, never media bytes | Accepted |
| [ADR-009](ADR-009-node-profile.md) | Supported node profile: 2 vCPU, 2 GB, 20 GB | Accepted |
| [ADR-010](ADR-010-upload-policy.md) | Default upload policy is review | Accepted |
| [ADR-011](ADR-011-anti-abuse.md) | Anonymous-upload anti-abuse: token buckets, quotas, optional PoW | Accepted |
| [ADR-012](ADR-012-naming-and-protocol.md) | Weft naming and the Weft Media Protocol (WMP) | Accepted |
| [ADR-013](ADR-013-activitypub-signatures.md) | ActivityPub uses RSA-2048; WMP keeps Ed25519 | Accepted |
| [ADR-014](ADR-014-jsonld-namespace.md) | JSON-LD namespace is `w3id.org/weft/ns#` | Accepted |

## Creating an ADR

1. Copy [`TEMPLATE.md`](TEMPLATE.md) to `ADR-<next>-<slug>.md`.
2. Fill in context, decision, consequences and alternatives.
3. Add it to the index with status `Proposed`.
4. After review, change the status to `Accepted`.
5. If it replaces an old decision, mark the old one `Superseded by ADR-XXX` and
   link the new ADR.
