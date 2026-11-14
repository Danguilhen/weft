# Weft — data model

Status: design for the standalone MVP. PostgreSQL 16 or later.

The schema is a proposal; the first migration must be reviewed against this
document and the API.

## 1. Principles

- Bytes and metadata are separate. `blobs` owns immutable bytes; `content` owns
  mutable local metadata.
- Blobs are deduplicated by SHA-256.
- The same hash may have different metadata on different nodes. On one node, the
  MVP keeps one local `content` row per blob and allows extra rows later.
- Removed content keeps a tombstone; the blob may be deleted when no active
  content references it.
- No raw IP addresses. Rate limiting uses a salted, rotating hash with a short
  retention.
- All times are `timestamptz`.

## 2. Tables

### `node`

A single row describing this node.

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | Local node UUID |
| `actor_url` | `text` unique | Stable ActivityPub identity |
| `public_url` | `text` | Canonical origin |
| `name` | `text` | Display name |
| `description` | `text` | Optional |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |

### `node_keys`

| Column | Type | Notes |
|---|---|---|
| `key_id` | `text` PK | Key URL or URN |
| `node_id` | `uuid` FK → `node` | |
| `alg` | `text` | `Ed25519` |
| `public_key` | `bytea` | Raw public key |
| `private_key_ref` | `text` | File path or secret-manager reference, never the key |
| `created_at` | `timestamptz` | |
| `valid_until` | `timestamptz` | Nullable |
| `revoked_at` | `timestamptz` | Nullable |

Private keys are not stored in the database in plaintext. The MVP may use a
root-only key file; a secret manager is a later option.

### `blobs`

Immutable bytes and their technical metadata.

| Column | Type | Notes |
|---|---|---|
| `hash` | `char(64)` PK | Lowercase SHA-256 |
| `size` | `bigint` | Exact original size |
| `mime` | `text` | Sniffed MIME |
| `width` | `integer` | Nullable |
| `height` | `integer` | Nullable |
| `frames` | `integer` | Nullable; GIF frame count |
| `duration_ms` | `integer` | Nullable |
| `storage_key` | `text` | Content-addressed path |
| `thumbnail_key` | `text` | Nullable |
| `created_at` | `timestamptz` | |
| `verified_at` | `timestamptz` | Last successful hash verification |

Indexes: PK; `mime`; `created_at`.

### `content`

Local listing metadata.

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | Local listing ID |
| `hash` | `char(64)` FK → `blobs` | |
| `title` | `text` | |
| `description` | `text` | |
| `origin_node` | `text` | Node actor URL that first published it |
| `status` | `content_status` | |
| `licence_spdx` | `text` | Nullable |
| `licence_url` | `text` | Nullable |
| `licence_claim` | `text` | Free-text claim |
| `licence_status` | `licence_status` | |
| `created_at` | `timestamptz` | |
| `updated_at` | `timestamptz` | |
| `published_at` | `timestamptz` | Nullable |
| `removed_at` | `timestamptz` | Nullable |
| `tombstone_reason` | `text` | Nullable |
| `search` | `tsvector` | Generated from title/description/tags |

Constraints: one active `content` row per `(hash, origin_node)` in the MVP.
Indexes: `hash`, `status`, `published_at DESC`, GIN on `search`.

### `content_tags`

| Column | Type | Notes |
|---|---|---|
| `content_id` | `uuid` FK → `content` | PK part |
| `tag` | `text` | PK part, normalised lowercase |

Indexes: `tag`; GIN for prefix matching if needed (`pg_trgm`).

### `content_sources`

Known locations for a content hash.

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | |
| `hash` | `char(64)` FK → `blobs` | |
| `node_id` | `text` | Node actor URL |
| `url` | `text` | Media URL |
| `bytes` | `bigint` | Nullable |
| `verified_at` | `timestamptz` | Nullable |
| `last_error` | `text` | Nullable |
| `created_at` | `timestamptz` | |

Unique `(hash, node_id)`. Indexes: `hash`, `verified_at`.

### `replicas`

Verified local or remote copies.

| Column | Type | Notes |
|---|---|---|
| `hash` | `char(64)` FK → `blobs` | PK part |
| `node_id` | `text` | PK part |
| `url` | `text` | |
| `bytes` | `bigint` | |
| `verified_at` | `timestamptz` | |
| `last_checked_at` | `timestamptz` | |
| `healthy` | `boolean` | |
| `expires_at` | `timestamptz` | Nullable |

Indexes: `(hash, healthy)`, `node_id`.

### `licence_evidence`

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | |
| `content_id` | `uuid` FK → `content` | |
| `url` | `text` | Evidence URL |
| `note` | `text` | Optional |
| `fetched_at` | `timestamptz` | Nullable |
| `fetched_hash` | `char(64)` | Hash of the fetched evidence snapshot |
| `created_at` | `timestamptz` | |

### `uploads`

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | Upload ID |
| `hash` | `char(64)` | Nullable until validation completes |
| `status` | `upload_status` | |
| `review` | `review_status` | |
| `client_hash` | `bytea` | Salted IP hash, short retention |
| `created_at` | `timestamptz` | |
| `reviewed_at` | `timestamptz` | Nullable |
| `detail` | `jsonb` | Validation results, no raw IP |

### `reports`

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | |
| `content_id` | `uuid` FK → `content` | Nullable if only a hash is known |
| `hash` | `char(64)` | |
| `reason` | `text` | `licence`, `illegal`, `spam`, `malware`, `privacy`, `other` |
| `details` | `text` | |
| `contact_hash` | `bytea` | Optional, hashed, short retention |
| `status` | `report_status` | |
| `created_at` | `timestamptz` | |
| `resolved_at` | `timestamptz` | Nullable |

### `moderation_events`

Append-only local log; the same payload can be published as an ActivityPub
`weft:ModerationEvent`.

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | Also the event ID |
| `content_id` | `uuid` FK → `content` | Nullable for node-level events |
| `hash` | `char(64)` | Nullable |
| `category` | `moderation_category` | |
| `scope` | `text` | `local`, `federated`, `global-intent` |
| `issuer` | `text` | Node actor URL |
| `reason` | `text` | |
| `evidence` | `jsonb` | URLs/references, no illegal samples |
| `payload` | `jsonb` | Full signed event |
| `signature` | `text` | Base64url |
| `created_at` | `timestamptz` | |
| `published_at` | `timestamptz` | Nullable |

Indexes: `(content_id, created_at)`, `category`, `created_at`.

### `receipts`

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | Receipt ID |
| `hash` | `char(64)` | |
| `served_by` | `text` | Node actor URL |
| `served_to` | `text` | Node actor URL |
| `bytes` | `bigint` | |
| `started_at` | `timestamptz` | |
| `ended_at` | `timestamptz` | |
| `nonce` | `bytea` | Single-use |
| `signature` | `text` | Server signature |
| `countersignature` | `text` | Nullable |
| `created_at` | `timestamptz` | |
| `valid` | `boolean` | |

Unique `(served_by, nonce)`. Indexes: `(served_by, created_at)`,
`(served_to, created_at)`, `hash`.

### `nodes` (remote peers)

| Column | Type | Notes |
|---|---|---|
| `node_id` | `text` PK | Actor URL |
| `public_url` | `text` | |
| `name` | `text` | |
| `descriptor` | `jsonb` | Last verified descriptor |
| `status` | `node_status` | `allowed`, `blocked`, `unknown` |
| `first_seen` | `timestamptz` | |
| `last_seen` | `timestamptz` | |
| `last_verified_at` | `timestamptz` | |

### `federation_outbox` and `federation_inbox` (Phase 2)

The outbox makes "change state" and "announce state" atomic. The inbox table is
the persistent replay/dedupe cache.

`federation_outbox`:

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | |
| `activity_id` | `text` unique | Signed activity ID |
| `kind` | `text` | `create`, `update`, `delete`, `flag`, `moderation` |
| `target` | `text` | Inbox URL |
| `payload` | `jsonb` | Signed activity |
| `state` | `outbox_state` | `queued`, `sending`, `sent`, `failed`, `dead` |
| `attempts` | `integer` | |
| `next_attempt_at` | `timestamptz` | |
| `last_error` | `text` | Redacted |
| `created_at` | `timestamptz` | |
| `sent_at` | `timestamptz` | Nullable |

Indexes: `(state, next_attempt_at)`, `activity_id`.

`federation_inbox`:

| Column | Type | Notes |
|---|---|---|
| `activity_id` | `text` PK | Replay/dedupe key |
| `actor` | `text` | |
| `received_at` | `timestamptz` | |
| `digest` | `text` | Request signature/digest fingerprint |
| `result` | `text` | `accepted`, `ignored`, `rejected` |
| `expires_at` | `timestamptz` | Replay window retention |

Indexes: `received_at`, `expires_at`.

Rules:

- The content/moderation change and its outbox row are inserted in the same
  transaction.
- A publisher worker sends queued rows with exponential backoff and a dead state.
- An inbound activity is inserted into the inbox before processing; a duplicate
  is acknowledged but not applied again.

### `node_blocks` / `url_blocks` / `tag_blocks`


Local policy rules:

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | |
| `kind` | `text` | `node`, `domain`, `url_prefix`, `tag` |
| `value` | `text` | Normalised |
| `reason` | `text` | |
| `created_at` | `timestamptz` | |

### `audit_log`

| Column | Type | Notes |
|---|---|---|
| `id` | `uuid` PK | |
| `actor` | `text` | `admin`, `system`, node actor |
| `action` | `text` | |
| `target` | `text` | |
| `detail` | `jsonb` | Redacted |
| `created_at` | `timestamptz` | |

### `rate_limits` (optional)

If rate limits are persisted instead of kept in memory:

| Column | Type | Notes |
|---|---|---|
| `key_hash` | `bytea` | Salted per window |
| `window_start` | `timestamptz` | |
| `count` | `integer` | |
| `expires_at` | `timestamptz` | |

A local in-memory bucket with periodic cleanup is acceptable for the MVP.

## 3. Enums

```text
content_status      pending | active | restricted | removed | rejected | failed
licence_status      unverified | evidence_provided | verified_evidence | disputed | invalidated
upload_status       received | validating | stored | rejected | failed
review_status       queued | approved | rejected | not_required
report_status       open | reviewing | actioned | dismissed
moderation_category CONTENT_REPORTED | LICENCE_DISPUTED | CONTENT_RESTRICTED |
                    CONTENT_BLOCKED | CONTENT_REMOVED | CONTENT_REINSTATED |
                    NODE_SUSPENDED
node_status         unknown | allowed | blocked
outbox_state        queued | sending | sent | failed | dead
```

Use PostgreSQL enums or text with `CHECK` constraints. Text plus checks is easier
to evolve; choose one in the first migration and document it.

## 4. Lifecycle

### Upload

1. Create `uploads` row with status `received`.
2. Validate into a temporary file; compute the hash.
3. On success, insert or reuse `blobs`; insert `content` with `status=pending`
   if review is required, otherwise `active`; set `published_at`.
4. On failure, mark `uploads` rejected/failed and delete the temporary file.

### Deduplication

- If a blob already exists, reuse it and verify the stored bytes.
- If an active `content` row exists for the same `(hash, origin_node)`, return
  the existing listing instead of creating a duplicate.
- A quarantined duplicate remains quarantined; do not leak it by returning the
  hash to an anonymous uploader beyond the upload response.

### Approval

- `status` → `active`, `published_at` set, search vector updated.
- Write a `moderation_events` row of category `CONTENT_REINSTATED` or a local
  approval audit entry (choose one; do not duplicate events).

### Rejection

- `status` → `rejected`; delete the blob if no other content references it.
- Keep the upload record for abuse handling without the media bytes.

### Removal / restriction

- `status` → `removed` or `restricted`; set `removed_at`/reason.
- Stop serving bytes; keep the content row and hash as a tombstone.
- Delete the blob unless another active content row references it.
- Write a moderation event and, in Phase 2, publish it.

### Reinstatement

- `status` → `active`; clear `removed_at`; write an event.

## 5. Blob deletion

A blob may be deleted only when no `content` row references it in `active`,
`restricted` or `pending` state. Tombstoned rows keep the hash but not the bytes.
Run an orphan detector periodically and report, never delete silently.

## 6. Search

- `search` is a generated `tsvector` from title, description and tags.
- Use the `simple` text search configuration initially; language-specific
  stemming is a post-MVP improvement.
- Tags are normalised: lowercase, trim, collapse whitespace, reject empty and
  overlong values.
- Trending uses a separate local counter or a materialised view; do not put
  popularity in the content row if it changes frequently.

## 7. Privacy and retention

- `client_hash` and `contact_hash` use a rotating salt; document the salt
  rotation and delete rows after the retention window.
- `reports.details` may contain personal data; restrict access and set a
  retention period.
- `audit_log` must not contain request bodies, tokens or media bytes.
- Node descriptors and moderation events are public; nothing else is.

## 8. Migrations

- Forward-only SQL migrations under `migrations/`, applied by `weft migrate`.
- Never edit an applied migration; add a new one.
- Destructive changes require a backup step in `docs/OPERATIONS.md`.
- The first migration is `0001_init.sql`; the schema version is tracked in
  `_sqlx_migrations`.

## 9. Open points

- Text enums versus PostgreSQL enums.
- Whether `content` should allow multiple listings per hash in the MVP.
- The exact trending data source and retention.
- Whether receipts are stored before Phase 4.
- The retention defaults for reports and client hashes.
