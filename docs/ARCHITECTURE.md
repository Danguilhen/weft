# Weft — architecture

Status: design, Phase 0. This document describes the target architecture for the
MVP and the federation milestones. It is not a description of existing code.

## 1. Goals

- A standalone node that a small community can self-host.
- Federation without a central authority or mandatory registration.
- Content-addressed media with verifiable integrity.
- Anonymous browsing and contributions by default.
- Independent moderation and licence acceptance per node.
- Demand-driven replication that does not make the origin a bottleneck.
- A documented REST API and client library that other applications can reuse.

## 2. Non-goals

- A global search index or a single canonical library.
- A blockchain, token, cryptocurrency or transferable credit.
- Guaranteed deletion of copies after they have been downloaded.
- Global reputation or a mandatory trust authority.
- Real-time chat or a replacement for XMPP.
- Transcoding every upload into a proprietary format.

## 3. Two layers

```text
             Metadata and events (ActivityPub)
  ┌──────────────┐   Create / Update / Delete   ┌──────────────┐
  │   Node A     │◀────────────────────────────▶│   Node B     │
  │              │   Flag / ModerationEvent     │              │
  └──────┬───────┘                              └──────┬───────┘
         │                                             │
         │      Media distribution (HTTPS, pull)       │
         │  manifest, bytes, replicas, receipts        │
         └─────────────────────────────────────────────┘
```

The **metadata layer** uses ActivityPub because it already provides actors,
inboxes, discovery and signed activities. The **media layer** is a small HTTPS
protocol optimised for content addressing, streaming, verification and
replication. ActivityPub activities may link to media manifests; they never carry
large media bodies.

The node must work with federation disabled. In that mode the media layer and the
REST API are still fully functional.

## 4. Components

| Component | Responsibility |
|---|---|
| `weft-core` | Wire and domain types, SHA-256 helpers, licence states, canonical JSON, policy traits |
| `weft-db` | PostgreSQL schema, migrations, queries and transactions |
| `weft-storage` | Content-addressed blob store; local filesystem default, S3/MinIO optional |
| `weft-validation` | Format detection, decode limits, thumbnails, metadata stripping |
| `weft-federation` | ActivityPub actor, inbox/outbox, HTTP signatures, node discovery, events |
| `weft-server` | Axum HTTP surface: public API, upload, admin, metrics, health |
| `weft-client` | Typed client for search, retrieval, upload, report and node info (MIT) |
| `weft-cli` | `init`, `keygen`, `migrate`, `serve`, `verify`, `import`, `reindex` |

`weft-core` is dependency-light and UI-agnostic. It must not depend on Axum,
SQLx or the server crates. `weft-client` depends only on `weft-core`, an HTTP
client and serialisation.

## 5. Data flow

### Upload

```text
client ──multipart──▶ server ──▶ validation worker
                                 │ sniff + decode limits
                                 │ SHA-256(original bytes)
                                 │ thumbnail + metadata strip
                                 ▼
                           storage (blob + thumb)
                                 ▼
                           PostgreSQL (content row)
                                 ▼
                    review queue or active index
```

The original bytes are never re-encoded; the hash covers exactly what was
uploaded. Thumbnails are derivatives and are never used for content identity.
Metadata such as EXIF/GPS is not copied into derivatives and is not exposed by
the API.

### Browse and retrieve

```text
client ──GET /api/v1/search──▶ PostgreSQL FTS ──▶ JSON results
client ──GET /media/{hash}───▶ storage ──▶ bytes + Digest + ETag
client ──GET /media/{hash}/manifest──▶ signed metadata
```

The media endpoint is public for active content. Restricted or removed content
returns `404`/`410`; the tombstone remains in the database for moderation.

### Federation

```text
Node A publishes content ──Create(Document)──▶ Node B inbox
Node B verifies signature, policy and licence ──▶ indexes metadata (optional)
Node B fetches a manifest ──▶ verifies signature and hash claim
Node B decides whether to replicate ──▶ pull bytes, verify, advertise replica
```

Discovery never grants trust. A node can discover a peer and still refuse to
index, replicate or serve it.

## 6. Content identity

- `blob_hash` = SHA-256 of the original bytes, lowercase hex.
- Public identity: `urn:weft:sha256:<blob_hash>`.
- ActivityPub object IDs are HTTPS URLs; the URN is a property
  (`weft:contentId`), not the ActivityPub `id`.
- The bytes are immutable. Title, description, tags, licence claims and
  moderation state are metadata and can change under a signed `Update`.
- Different nodes may hold different metadata for the same hash. A node must not
  assume that another node's licence claim is its own policy decision.
- A content record has a local `content_id` separate from `blob_hash`, so a node
  can later support multiple listings of the same bytes without changing the
  wire identity.

## 7. Storage layout

Local backend:

```text
data/
  blobs/sha256/ab/cd/<hash>
  thumbs/<hash>.webp
  tmp/<upload-id>
  quarantine/<upload-id>
```

The blob path is derivable from the hash. Deduplication is a database lookup
plus an idempotent file write (write to a temporary file, verify the hash, then
rename). The storage trait hides local and S3 differences.

Quotas are enforced before and during a write. The node never trusts
`Content-Length` alone.

## 8. Search

The MVP uses PostgreSQL:

- `tsvector` over title, description and tags;
- `pg_trgm` for partial tag matches;
- deterministic cursor pagination;
- trending as a bounded, decaying score from local views and downloads.

A `SearchIndex` trait isolates handlers. Meilisearch can be added later as an
optional service, but the default deployment stays app + PostgreSQL.

## 9. Security boundaries

| Boundary | Trust |
|---|---|
| Internet → public API | Untrusted input; rate limit and validate |
| Upload bytes → decoder | Untrusted; resource-limited worker |
| Remote node → inbox | Untrusted; signatures, policy, replay checks |
| Remote manifest → local index | Untrusted; verify signature and hashes |
| Admin API | Bearer token; loopback or explicit allow-list |
| Database | Trusted but least-privilege role |
| Storage | Trusted for integrity only after hash verification |

Upload decoding is the highest-risk boundary. The design calls for a separate
worker process with time, memory and file-size limits. On Linux, a sandbox
(bubblewrap or a dedicated container) is preferred for public nodes.

## 10. Deployment

### Small node (default)

```text
reverse proxy (TLS)
  └── weft-server :8080
        ├── PostgreSQL :5432
        └── local volume /var/lib/weft
```

### Optional services

- S3/MinIO for blobs.
- Meilisearch for search.
- Prometheus scraping `/metrics`.
- A separate decode worker container.

Docker Compose is the reference deployment. The server itself does not terminate
TLS; the operator's reverse proxy does.

## 11. Failure modes

| Failure | Behaviour |
|---|---|
| Decoder timeout | Upload rejected; no blob committed |
| Disk full | Upload rejected; quota/health alert; existing content readable |
| PostgreSQL down | API returns `503`; media bytes may still be served if cached |
| Origin node offline | Replicas continue; manifests remain verifiable |
| Peer sends bad hash | Transfer rejected; source marked unhealthy |
| Key compromise | Rotate key, publish revocation, keep old public key for verification history |
| Corrupt blob | Hash check fails; quarantine and alert; do not serve |

## 12. Technology choices

Accepted in ADRs:

- Rust, Axum, Tokio, SQLx, PostgreSQL (ADR-002).
- PostgreSQL-first search (ADR-002).
- SHA-256 content identity (ADR-003).
- Pull replication and signed receipts (ADR-004).
- Anonymous uploads with a quarantine policy (ADR-005).
- AGPL server / MIT core and client (ADR-006).
- XEP-0363 delivery for the first XMPP client (ADR-007).
- ActivityPub for metadata only (ADR-008).

Not yet decided: public name/domain, JSON-LD namespace, Ed25519/RSA
interoperability, default upload policy, search backend after the MVP, Jabu send
mode. See [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md).

## 13. Assumptions

- A small node can run with PostgreSQL and local storage; no object store is
  required.
- Federation peers are Weft-aware; generic Mastodon interoperability is not a
  first-milestone requirement.
- Ed25519 signatures over canonical JSON are sufficient for the Weft protocol.
- GIF originals can be served without transcoding; derivatives are optional.
- Operators accept that a decentralised network cannot guarantee deletion of
  copies already downloaded.

## 14. Related documents

- [`PROTOCOL.md`](PROTOCOL.md) — wire protocol.
- [`FEDERATION.md`](FEDERATION.md) — ActivityPub objects and events.
- [`API.md`](API.md) — REST surface.
- [`DATA_MODEL.md`](DATA_MODEL.md) — PostgreSQL model.
- [`THREAT_MODEL.md`](THREAT_MODEL.md) — adversaries and mitigations.
- [`MVP.md`](MVP.md) — first implementation milestone.
