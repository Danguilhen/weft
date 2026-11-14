# Weft MVP — standalone node

This is the first implementation milestone. It must work without federation,
without a commercial GIF provider and without any central Weft service.

## Goal

A person can run one Weft node with Docker, upload a GIF anonymously, browse and
search the local library, and retrieve the original bytes with an integrity
check. An operator can moderate the node from a CLI or admin API.

## In scope

### Node and operations

- `weft init` creates the node identity, key material and a TOML config.
- `weft migrate` applies the PostgreSQL schema.
- `weft serve` runs the HTTP server.
- `weft keygen` and `weft key rotate` prepare node keys; rotation is stubbed
  until federation exists.
- `GET /healthz`, `GET /readyz`, `GET /metrics`.
- Container image and Docker Compose stack (app + PostgreSQL).
- A reverse-proxy example with TLS termination.
- Structured logs with redaction; no raw IPs in normal logs.

### Upload and validation

- Multipart upload endpoint with a configurable policy: `open`, `review` or
  `closed`.
- Anonymous uploads are rate-limited by a salted, rotating IP hash.
- Validate by decoding, not by extension or client MIME:
  - file size;
  - pixel count;
  - frame count for GIF;
  - decoded memory;
  - decode time;
  - format allow-list (GIF and static PNG/JPEG/WebP for the MVP).
- Compute SHA-256 over the original bytes.
- Strip metadata from derivatives and thumbnails; keep the original bytes
  unchanged for integrity.
- Store blobs in a content-addressed layout.
- Generate a thumbnail (or a safe placeholder).
- Quarantine new uploads unless policy says otherwise.
- Reject active content, SVG and anything the decoder does not recognise.

### Browse and retrieve

- `GET /api/v1/media` with cursor pagination, filters and sort (newest,
  trending).
- `GET /api/v1/search?q=` using PostgreSQL full-text search and tags.
- `GET /api/v1/trending`.
- `GET /api/v1/tags`.
- `GET /media/{hash}` — original bytes with `Content-Type`, `ETag`,
  `Digest: sha-256=...`, `Cache-Control` and Range support.
- `GET /api/v1/media/{hash}` — signed metadata (alias of the protocol manifest).
- `GET /api/v1/media/{hash}/sources` — known locations (only the local node in
  the MVP).

### Metadata and licensing

- Title, description, tags, source/attribution, created time.
- Licence claim with SPDX or Creative Commons identifiers.
- Licence evidence URLs, optional.
- Licence status: `unverified` by default; an operator can mark evidence as
  provided or verified.
- A content report endpoint and a licence-dispute endpoint that create a local
  moderation event.
- No claim that a licence is legally verified.

### Moderation

- Quarantine queue with approve/reject.
- Remove, restrict and reinstate content.
- Local block rules for URLs/domains and tags.
- A signed, append-only local moderation event log.
- A tombstone record for removed content.
- Admin CLI/API with a bearer token; no public admin UI in the MVP.

### Data and storage

- PostgreSQL for records and full-text search.
- Local filesystem for blobs and thumbnails.
- Optional S3/MinIO backend behind the same storage trait.
- Content cleanup: orphaned blob detection, eviction markers (automatic
  eviction is Phase 3; the MVP exposes quotas and a dry run).

### Documentation and tests

- OpenAPI document served at `/api/openapi.json`.
- A `docs/STATUS.md` update with what is verified.
- Unit tests for hashing, canonical JSON, validation limits and licence states.
- Integration tests with PostgreSQL for upload, search and retrieval.
- A fuzz target for the upload decoder boundary (at least a corpus-based test).
- A smoke script that starts the stack, uploads a GIF and verifies the hash.

## Out of scope

- ActivityPub federation and node discovery.
- Replication, receipts and contribution accounting.
- Meilisearch or any separate search service.
- A browser SPA or public web gallery.
- User accounts, passwords or OAuth.
- Android, desktop packaging or a Jabu picker.
- Automatic transcoding to WebP/AVIF.
- A global blocklist, global reputation or central licence authority.

## Acceptance demo

1. `docker compose up -d` starts the node and PostgreSQL.
2. `weft init` has created a node key and config.
3. Upload a GIF through `curl`; response is `202` with the SHA-256.
4. The content appears in `review`; an operator approves it.
5. `GET /api/v1/search?q=...` returns the item.
6. `GET /media/{hash}` returns the exact bytes; the hash matches.
7. `GET /api/v1/media/{hash}` verifies against the node key.
8. Removing the content makes search and media return `404`/`410` while keeping a
   tombstone.
9. `GET /metrics` exposes upload, decode, storage and request counters.
10. `python3 tools/check_docs.py` and `cargo test --workspace` pass.

## Success metrics

- A 1 vCPU / 1 GB node can serve the demo with PostgreSQL and a 10 GB storage
  budget.
- No upload can make the node allocate unbounded memory or time.
- A failed validation leaves no blob and no searchable record.
- A deleted item is not returned by search or media endpoints.
- Logs contain no raw IP address or media bytes.

## Risks

| Risk | Mitigation |
|---|---|
| Decoder vulnerabilities | Limits, isolated worker, fuzzing, dependency updates |
| Spam uploads | Quarantine default, per-IP buckets, optional proof of work |
| Copyright claims | Explicit unverified status, evidence, disputes, local policy |
| PostgreSQL-only search is weak | Keep a search trait; evaluate Meilisearch after MVP |
| Small-node resource pressure | Hard limits, thumbnails only, no transcoding by default |
