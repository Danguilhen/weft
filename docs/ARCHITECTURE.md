# Weft — architecture

Status: **design for review**. This document specifies the target architecture for
the standalone MVP and the federation milestones. It is not a description of
existing code. Implementation is not authorised until
[`DESIGN_REVIEW.md`](DESIGN_REVIEW.md) records an approval.

Related decisions: [`adr/README.md`](adr/README.md). Open choices:
[`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md). Security analysis:
[`THREAT_MODEL.md`](THREAT_MODEL.md).

## 1. Scope

### Goals

- A standalone node a small community can self-host.
- Federation without a central authority or mandatory registration.
- Content-addressed media with verifiable integrity.
- Anonymous browsing and contributions by default.
- Independent moderation and licence acceptance per node.
- Demand-driven replication that does not make the origin a bottleneck.
- A documented REST API and client library other applications can reuse.

### Non-goals

- A global search index or a single canonical library.
- A blockchain, token, cryptocurrency or transferable credit.
- Guaranteed deletion of copies already downloaded.
- Global reputation or a mandatory trust authority.
- Real-time chat or a replacement for XMPP.
- Transcoding every upload into a proprietary format.
- A browser SPA, mobile app or desktop packaging in the MVP.

## 2. System context

```text
┌─────────────┐   HTTPS/JSON, /media   ┌──────────────────────────────┐
│ Clients     │───────────────────────▶│  Weft node                   │
│ Jabu, curl, │◀───────────────────────│  reverse proxy → weft-server │
│ other apps  │                        │      │                       │
└─────────────┘                        │      ├── PostgreSQL          │
                                       │      ├── blob storage        │
┌─────────────┐  ActivityPub (metadata)│      └── weft-decode worker  │
│ Peer node   │◀──────────────────────▶│                              │
│             │  media protocol (bytes)│                              │
└─────────────┘◀──────────────────────▶└──────────────────────────────┘
```

The node is independently operable. Federation is optional; the REST API and the
media protocol are fully functional without it. There is no central Weft service
and no mandatory bootstrap node.

## 3. Two layers

```text
Metadata and events (ActivityPub)
  Node A  ◀──── Create / Update / Delete / Flag / ModerationEvent ────▶  Node B
     │
     │  Media distribution (HTTPS, pull)
     │  manifests, immutable bytes, replica advertisements, receipts
     ▼
  Node B fetches bytes only after its own policy decision
```

- **Metadata layer:** activity delivery, discovery, announcements, licence and
  moderation events. Small JSON documents only.
- **Media layer:** manifests, original bytes, replica lists and transfer
  receipts. Cacheable and content-addressed.

ActivityPub activities may link to a manifest or a media URL; they never carry
the bytes. See ADR-001 and ADR-008.

## 4. Component map

| Component | Responsibility | May depend on |
|---|---|---|
| `weft-core` | Domain and wire types, SHA-256 helpers, canonical JSON, licence/moderation states, policy traits | Serde, crypto primitives only |
| `weft-db` | PostgreSQL schema, migrations, repositories, transactions | `weft-core`, SQLx |
| `weft-storage` | Content-addressed blob store, thumbnails, quotas, orphan scan | `weft-core` |
| `weft-validation` | Format sniffing, decode limits, thumbnails, metadata stripping; `weft-decode` worker binary | `weft-core` |
| `weft-federation` | Actor, inbox/outbox, signatures, discovery, events | `weft-core`, `weft-db`, HTTP client |
| `weft-server` | Axum routers, auth, rate limits, application services, metrics | All server crates |
| `weft-client` | Typed REST client for Jabu and other apps | `weft-core`, HTTP client |
| `weft-cli` | `init`, `keygen`, `migrate`, `serve`, `verify`, `import`, `reindex` | Server crates |

Rules:

- `weft-core` and `weft-client` are MIT and must not depend on server crates.
- `weft-core` must not depend on Axum, SQLx, Tokio or a UI framework.
- `weft-validation` must not access the database or the network.
- `weft-decode` is a separate process, not a thread in the web server.
- Application services live in `weft-server` for the MVP; if they grow, they move
  to a `weft-app` crate without changing the wire surface.

## 5. Domain model and identity

### Identities

| Identity | Definition | Stability |
|---|---|---|
| Node ID | ActivityPub actor URL, e.g. `https://node.example/actor` | Stable across key rotation |
| Key ID | URL fragment, e.g. `https://node.example/actor#key-1` | Rotates; old keys retained |
| Content ID | `urn:weft:sha256:<hash>` | Immutable, derived from bytes |
| Listing ID | Local UUID | Local; may change |
| Contributor ID | Optional pseudonymous key (later) | Optional, not federated by default |

A node's private key is a root-only file or secret-manager value, never a
database column or a log line. The admin token is a random secret stored hashed
(Argon2id) with no cookie-based session.

### Content and metadata

- `blob_hash` = SHA-256 over the original uploaded bytes.
- Bytes are immutable. Title, description, tags, licence claim and moderation
  state are local metadata and may change under a signed `Update`.
- Different nodes may hold different metadata and licence claims for the same
  hash. A node's policy applies to its own listing, not to the bytes globally.
- A local `content` listing is separate from the blob, so multiple listings per
  hash are possible later without changing the content ID.

## 6. Upload pipeline

### Sequence

```text
HTTP multipart
  │  body limit, timeout, request ID
  ▼
weft-server ──stream──▶ temp file (0600, private temp dir)
  │  hash while streaming; enforce max bytes
  ▼
sniff format in server (defence in depth)
  ▼
weft-decode worker (separate process)
  │  fd/path, rlimits, timeout, no network/DB
  │  decode, count frames, check pixels/memory
  │  write thumbnail to a private temp file
  ▼
server cross-checks sniffed MIME and dimensions
  ▼
PostgreSQL transaction:
  upsert blob; insert content pending/active; insert tags; insert upload
  ▼
atomic rename temp → blobs/sha256/ab/cd/<hash>
  ▼
202 { uploadId, contentId, status }
```

### Isolation

The decoder is a security boundary and must not run in the HTTP server process.

- `weft-decode` is spawned by the server or runs as a sidecar container.
- Communication is a Unix socket; the server passes the media file descriptor
  with `SCM_RIGHTS` where supported. A private 0600 path is the fallback.
- The worker has no network, no database and no access to the storage root except
  the passed descriptor and its own temp directory.
- The worker sets `RLIMIT_AS`, `RLIMIT_CPU`, `RLIMIT_FSIZE`, `RLIMIT_NOFILE` and
  `PR_SET_NO_NEW_PRIVS`; the server enforces a wall-clock timeout and kills the
  process group on expiry.
- On Linux, public nodes should run the worker under bubblewrap or a dedicated
  container with a read-only root filesystem.
- The MVP must ship the worker even if the default deployment runs it with soft
  limits; the in-process-only path is not acceptable for a public node.

### Transactions and durability

1. The blob is written to a private temp file, hashed and `fsync`ed.
2. The thumbnail is generated and written to a separate temp file.
3. The database transaction commits `blobs`, `content`, `content_tags` and
   `uploads`.
4. The blob is atomically renamed into its content-addressed path and the
   directory is `fsync`ed.
5. If the database commit fails, temp files are removed. If the rename fails,
   the transaction is rolled back.
6. If the process dies between commit and rename, an orphan scanner detects the
   missing blob and either restores it from temp or marks the content failed.

Uploads may carry an `Idempotency-Key`; a repeat returns the same upload/content
ID instead of creating a duplicate. This is an API extension to add before the
MVP is declared complete.

## 7. Retrieval and manifests

- `GET /media/{hash}` streams the original bytes with `Content-Type`, `ETag`,
  `Digest`, `Cache-Control`, `Accept-Ranges` and `X-Content-Type-Options:
  nosniff`. Range requests are supported.
- Active content is public. Quarantined content is `404`; removed content is
  `410` with a tombstone reference.
- `GET /media/{hash}/manifest` returns the signed manifest. The API alias is
  `GET /api/v1/media/{hash}`.
- Manifests are signed with the node key over RFC 8785 canonical JSON. The
  signature field is excluded from the signing input.
- Blobs are verified when written and during a background scrub. A corrupt blob
  is quarantined and not served.
- Thumbnails are derivatives, not identity; they are regenerated if missing and
  never substituted for the original in a media response.

## 8. Search and discovery

- PostgreSQL FTS (`websearch_to_tsquery`) over title, description and tags; no
  string interpolation into SQL.
- `pg_trgm` for tag prefix matching.
- Deterministic cursor pagination; a hard limit (100) per page.
- Trending is a local, decaying score from views and downloads over a window;
  there is no global ranking.
- The `SearchIndex` trait isolates the handler so Meilisearch can replace the
  backend without changing the API.
- Search queries are not logged and are not federated.

## 9. Federation

### Actor and discovery

- One `Service` actor per node, with Ed25519 keys and a signed descriptor at
  `/.well-known/weft`.
- Discovery via bootstrap list, peer exchange, ActivityPub references and manual
  configuration. Discovery grants no trust.
- A pinned local JSON-LD context; **remote contexts are never fetched**. Parse a
  fixed JSON shape and reject unknown or unsafe constructs.
- Outbound fetches allow HTTPS only, reject private/link-local addresses, limit
  redirects and enforce timeouts.

### Activities

- `Create`, `Update`, `Delete` for content metadata.
- `Flag` for reports.
- `weft:ModerationEvent` for moderation/licence decisions.
- `Block` for a local statement of defederation.
- No media bytes in any activity.

### Delivery

- Use a transactional outbox: the content/moderation change and the outbox row
  commit in the same database transaction.
- A background publisher signs and delivers with exponential backoff and a
  dead-letter state.
- Inbound activities are deduplicated by activity ID in a persistent inbox table.
- A remote node's decision is input to local policy, never an automatic command.

## 10. Replication

- Pull only. A node decides to replicate; no peer can force storage.
- Demand is tracked locally per content; popularity, storage headroom, bandwidth
  and existing replica count feed a local candidate score.
- A source is selected from verified replicas or the origin, fetched with a
  byte cap and timeout, hashed, and atomically committed.
- Failed sources are marked unhealthy with backoff; a bad hash never reaches the
  cache.
- Quotas: global, per-file, per-content, per-peer, bandwidth and concurrency.
- Eviction: LRU/LFU with pinning for local uploads and recently requested items;
  never evict content with an active local listing without a policy decision.
- Replica advertisements are signed hints; the receiver verifies by fetching.

## 11. Resource accounting (Phase 4)

- Signed receipts for completed transfers, countersigned by the requester.
- Local scoring from verified bytes, stored replicas and availability.
- Diminishing returns for repeated content/peer transfers; same-operator
  transfers do not count as independent contributions.
- Scores influence local quotas and replication placement only. Basic browsing
  and download never require a high score.
- No global ledger, transferable credit or consensus.

## 12. Moderation and licensing

- Quarantine → approve/reject; active → restrict/remove/reinstate; tombstones kept.
- Licence statuses are explicit; unverified claims are never shown as verified.
- Reports and licence disputes create local events and are rate-limited.
- Moderation events are signed and may be published; other nodes apply their own
  policy.
- Illegal content is handled locally with a no-sample-federation rule.
- Admin actions are audit-logged without bodies, tokens or raw addresses.

## 13. Storage and data lifecycle

```text
data/
  blobs/sha256/ab/cd/<hash>     # original bytes, immutable
  thumbs/<hash>.webp            # derivative, regenerable
  tmp/<upload-id>/              # private, 0600, cleaned on failure
  quarantine/<upload-id>/       # optional staging before approval
```

- Paths are derived from hashes, never from client filenames.
- Open files with `O_NOFOLLOW` where supported; the storage root is not
  world-writable; reject symlinks and unexpected ownership.
- Blob deletion occurs only when no `content` row references it in an active,
  restricted or pending state; tombstones keep the hash but not the bytes.
- Orphan detection is a report by default; deletion is explicit.
- Thumbnails can be rebuilt; they are not backed up if storage is tight.

## 14. Consistency and transactions

- PostgreSQL is the metadata source of truth; blobs are immutable content.
- One transaction per state change (upload, approve, remove, report).
- Blob write → DB commit → atomic rename; recovery uses temp files and orphans.
- Federation uses a transactional outbox; inbox uses a persistent dedupe table.
- Receipts and moderation events are append-only; corrections are new events.
- All state-changing endpoints are idempotent or accept an idempotency key.

## 15. Security architecture

| Boundary | Rule |
|---|---|
| Internet → API | Untrusted input; body/time/size limits; rate limits; no stack traces |
| Upload → decoder | Separate process; rlimits; no network/DB; timeout and kill |
| Remote activity → inbox | Verify signatures; pin context; SSRF-safe fetch; dedupe |
| Remote manifest → index | Verify signature and hashes; local policy |
| Admin API | Bearer token, hashed at rest; loopback or allow-list; no cookies |
| Database | Least-privilege role; localhost by default |
| Storage | Verify hash before serving; no symlinks; private temp dirs |
| Federation egress | HTTPS only; reject private/link-local; redirect/timeout limits |

Additional rules:

- No cookies for authentication; bearer headers avoid CSRF by construction.
- CORS is disabled by default; an explicit allow-list is required for browser
  clients.
- JSON responses are data; clients escape them. An admin HTML page, if added,
  needs a strict CSP.
- `X-Content-Type-Options: nosniff` on media; no SVG/HTML uploads.
- Secrets are env/root-only files; logs redact URLs with credentials.
- Error `detail` must not expose filesystem paths or internal identifiers.

## 16. Deployment

### Small node (reference)

```text
reverse proxy (TLS, body limit, HSTS)
  └── weft-server :8080
        ├── weft-decode (same image, worker mode; sandbox optional in dev)
        ├── PostgreSQL :5432
        └── /var/lib/weft (blobs, thumbs, key, salt)
```

### Optional services

- S3/MinIO for blobs.
- Separate decode worker container with a read-only root and no network.
- Meilisearch for search.
- Prometheus scraping `/metrics`.

The app does not terminate TLS. Compose is the reference deployment. Federation
and replication are disabled by default.

## 17. Capacity and small-node profile

Illustrative, to be measured in the MVP:

| Profile | CPU | RAM | Storage | Limits |
|---|---|---|---|---|
| Minimum (read-mostly, small library) | 1 vCPU | 1 GB | 10 GB | 128 MB decode, 8 MP, 150 frames |
| Recommended public node | 2 vCPU | 2 GB | 20 GB | 256 MB decode, 16 MP, 300 frames |
| Replicating node | 2 vCPU | 2 GB | 50 GB+ | quotas per policy |

- Metadata is small (a few KB per item) plus FTS; thumbnails ~30–80 KB.
- Blobs dominate storage. Budget 20% free for temp/WAL.
- One decode per vCPU; concurrency is capped, not queued without bound.
- 1 Mbps sustained egress is about 10 GB/day; form a realistic budget.
- A 1 GB node with PostgreSQL and a 256 MB decode budget is tight; the review
  recommends 2 GB for a public node or the minimum limits above.

## 18. Failure modes

| Failure | Behaviour |
|---|---|
| Decode timeout/crash | Upload rejected; no blob committed; worker restarted |
| Disk full | Upload rejected; health alert; existing content readable |
| PostgreSQL down | API `503`; bytes may still be served if already in cache |
| DB commit after blob write fails | Temp blob removed; no listing |
| Rename fails after commit | Orphan detector; content marked failed until repaired |
| Corrupt blob | Hash check fails; quarantine; alert; never serve |
| Origin node offline | Replicas continue; manifests remain verifiable |
| Bad replica hash | Source marked unhealthy; transfer rejected |
| Federation outbox stuck | Retry/backoff; dead-letter; local state unaffected |
| Key compromise | Rotate, publish revocation, keep old keys for history |
| Search backend down | Optional Meili degradation; Postgres remains the default |
| Revoked/expired admin token | Requests rejected; rotation via CLI |

## 19. Observability

- `GET /healthz`, `GET /readyz`, `GET /metrics`.
- Request ID on every request; structured JSON logs; redaction.
- Counters: uploads received/accepted/rejected, decode failures by reason,
  bytes stored/served, search latency, queue depth, outbox depth, replica
  failures, 4xx/5xx.
- Gauges: disk usage, DB connections, quarantine size, orphan count.
- No raw IPs, tokens, search queries or media bodies in logs or labels.
- Alerts: disk > 80%, DB down, decode failure spike, outbox backlog, key expiry.

## 20. MVP versus later

| Capability | MVP | Later |
|---|---|---|
| Upload, validate, hash, store | Yes | — |
| Review queue, approve/reject | Yes | — |
| Search/trending/tags, manifests, bytes | Yes | — |
| Reports, licence claims/disputes (local) | Yes | Federated events |
| Decoder worker | Yes | Sandboxed container by default |
| ActivityPub discovery/announcements | No | Phase 2 |
| Replication and replicas | No | Phase 3 |
| Receipts and accounting | No | Phase 4 |
| Meilisearch | No | Optional |
| Jabu GIF picker | No | Phase 6 |

## 21. Requirements traceability

| Brief area | Owner document |
|---|---|
| Two layers, federation and media | This document, ADR-001, ADR-008 |
| Node identity, keys, rotation | This document, `FEDERATION.md` |
| Anonymous contributions and privacy | `PRIVACY.md`, ADR-005 |
| Content identity and metadata | ADR-003, `DATA_MODEL.md` |
| Demand-driven replication | `PROTOCOL.md`, ADR-004 |
| Resource contribution accounting | `PROTOCOL.md`, `PLAN.md` |
| Upload validation and abuse | `THREAT_MODEL.md`, `MVP.md` |
| Licence claims and disputes | `CONTENT_LICENSING.md` |
| Moderation events | `FEDERATION.md` |
| REST API | `API.md`, `openapi.yaml` |
| XMPP/Gabu integration | `XMPP.md`, ADR-007 |
| Deliverables and phases | `PLAN.md`, `STATUS.md` |

## 22. Accepted decisions and open questions

Accepted: see [`adr/README.md`](adr/README.md).

Blocking or review-relevant open questions:
- Q1/Q2: public name, domain and JSON-LD namespace (federation).
- Q3: Ed25519-only vs RSA compatibility.
- Q4/Q7: default upload policy and anti-abuse.
- Q5: search backend after the MVP.
- Q6: Jabu upload vs link mode.
- Q11: minimum node size / 1 GB feasibility.

See [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) for the full list.

## 23. Related documents

- [`THREAT_MODEL.md`](THREAT_MODEL.md) — adversaries, threats, mitigations.
- [`DESIGN_REVIEW.md`](DESIGN_REVIEW.md) — review findings and approval state.
- [`PROTOCOL.md`](PROTOCOL.md) — media wire protocol.
- [`FEDERATION.md`](FEDERATION.md) — ActivityPub objects and events.
- [`API.md`](API.md) — REST surface.
- [`DATA_MODEL.md`](DATA_MODEL.md) — PostgreSQL model.
- [`MVP.md`](MVP.md) — first implementation milestone.
- [`OPERATIONS.md`](OPERATIONS.md) — deployment and maintenance.
