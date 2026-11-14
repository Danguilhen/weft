# Weft — REST API v1

Status: draft for the standalone MVP and the first federation milestones.

Base URL: the node origin, e.g. `https://gifs.example`. JSON endpoints are under
`/api/v1`. Immutable media bytes are served from `/media/{hash}` so they can be
cached independently of the API version.

Machine-readable specification: [`openapi.yaml`](openapi.yaml). Wire-level
details: [`PROTOCOL.md`](PROTOCOL.md).

## 1. Conventions

- JSON requests and responses use `application/json; charset=utf-8`.
- Uploads use `multipart/form-data`.
- Times are RFC 3339 UTC.
- Hashes are lowercase SHA-256 hex.
- Cursor pagination: pass the opaque `cursor` from `nextCursor`; never construct
  one by hand.
- Unknown response fields must be ignored by clients.
- Errors use the shape in §9.
- Public read endpoints require no account and set no tracking cookies.

## 2. Authentication

| Surface | Authentication |
|---|---|
| Search, trending, tags, media bytes, manifests, sources | None |
| Upload | Anonymous when policy allows, otherwise an admin bearer token |
| Report, licence dispute | None; rate-limited |
| Admin endpoints | `Authorization: Bearer <token>` |
| Node-to-node replication/receipts | Ed25519 signed request headers |

Anonymous uploads are rate-limited by a salted, rotating hash of the client
address. Raw addresses are not stored.

## 3. Content summary

Search, trending and list endpoints return:

```json
{
  "hash": "3a7bd3e2360a...",
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "mime": "image/gif",
  "width": 480,
  "height": 270,
  "frames": 42,
  "durationMs": 3200,
  "title": "Surprised cat",
  "tags": ["cat", "reaction"],
  "licence": {
    "spdx": "CC0-1.0",
    "status": "unverified"
  },
  "mediaUrl": "https://gifs.example/media/3a7bd3e2360a...",
  "thumbnailUrl": "https://gifs.example/media/3a7bd3e2360a.../thumbnail",
  "manifestUrl": "https://gifs.example/media/3a7bd3e2360a.../manifest",
  "createdAt": "2026-11-14T00:00:00Z"
}
```

Restricted or removed content is not returned by public search.

## 4. Discovery endpoints

### `GET /api/v1/search`

Query parameters:

| Name | Type | Default | Notes |
|---|---|---|---|
| `q` | string | — | Free text; empty returns the newest items |
| `tags` | comma-separated | — | AND semantics |
| `mime` | string | — | `image/gif`, `image/webp`, etc. |
| `sort` | enum | `relevance` | `relevance`, `newest`, `trending` |
| `cursor` | string | — | Opaque |
| `limit` | integer | 20 | 1–100 |

Response:

```json
{
  "items": [ /* content summaries */ ],
  "nextCursor": "opaque-or-null",
  "totalEstimate": 42
}
```

`totalEstimate` is optional and may be approximate.

### `GET /api/v1/trending`

Parameters: `window` (`24h`, `7d`, `30d`; default `24h`), `cursor`, `limit`.
Trending is calculated locally from views and downloads. It is not a global
ranking.

### `GET /api/v1/media`

Newest or cursor-paginated listing with `sort` (`newest`, `trending`), `tag`,
`cursor` and `limit`.

### `GET /api/v1/tags`

Returns the local tag vocabulary with counts:

```json
{
  "items": [{ "tag": "cat", "count": 12 }],
  "nextCursor": null
}
```

### `GET /api/v1/media/{hash}`

Returns the signed manifest. This is an API convenience alias for
`GET /media/{hash}/manifest`; the bytes are at `GET /media/{hash}`.

### `GET /api/v1/media/{hash}/sources`

Returns known replicas and their verification times. Sources are hints, not
trust.

### `GET /media/{hash}/thumbnail`

Returns the stored JPEG thumbnail. It is intended for picker grids and
previews; the original bytes remain at `/media/{hash}`. The response is
immutable and cached for a year.

### `GET /media/{hash}`

Returns the original bytes. Headers: `Content-Type`, `Content-Length`, `ETag`,
`Digest`, `Cache-Control`, `Accept-Ranges`, `X-Content-Type-Options: nosniff`.
Supports `HEAD` and `Range`. `404` for unknown or quarantined content, `410` for
removed content.

### `GET /media/{hash}/manifest` and `GET /media/{hash}/sources`

Protocol aliases for the API endpoints above.

## 5. Upload

### `POST /api/v1/upload`

`multipart/form-data` fields:

| Field | Required | Notes |
|---|---|---|
| `file` | yes | One file. GIF or allowed static image. |
| `title` | no | Plain text, length-limited |
| `description` | no | Plain text, length-limited |
| `tags` | no | Comma-separated, normalised |
| `source` | no | Original URL or attribution |
| `licence` | no | SPDX or Creative Commons identifier |
| `licenceUrl` | no | Licence URL |
| `evidence` | no | One evidence URL, repeatable |

Response `202`:

```json
{
  "uploadId": "01J...",
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "hash": "3a7bd3e2360a...",
  "status": "pending",
  "review": "queued",
  "manifestUrl": "https://gifs.example/api/v1/media/3a7bd3e2360a..."
}
```

Status values: `pending`, `active`, `rejected`, `failed`.

Errors:

- `413 payload_too_large`
- `415 unsupported_media_type`
- `422 validation_failed` with a machine-readable `detail`
- `429 rate_limited` with `Retry-After`

The bytes are not public until the content is approved, unless policy is `open`.

An optional `Idempotency-Key` request header makes retries safe: a repeated
upload with the same key returns the same `uploadId` and `contentId` instead of
creating a duplicate.

## 6. Reports and licence disputes

### `POST /api/v1/report`

```json
{
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "reason": "licence|illegal|spam|malware|privacy|other",
  "details": "Plain text, optional",
  "contact": "optional, never published"
}
```

Response `202` with a local report ID. Reports are rate-limited. A report is not
a moderation decision and does not federate automatically.

### `POST /api/v1/licence-dispute`

```json
{
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "claim": "The claimed licence is incorrect.",
  "evidence": ["https://example.org/original"],
  "contact": "optional"
}
```

Creates a local `LICENCE_DISPUTED` event. The content may be restricted while
the operator reviews it.

## 7. Node and federation endpoints

| Method | Path | Auth | Purpose |
|---|---|---|---|
| `GET` | `/api/v1/nodes` | none | Known node descriptors (paginated) |
| `GET` | `/api/v1/nodes/{nodeId}` | none | One descriptor |
| `POST` | `/api/v1/replicas` | signature | Advertise a verified replica |
| `POST` | `/api/v1/receipts` | signature | Submit a countersigned transfer receipt |
| `GET` | `/api/v1/node` | none | This node's descriptor |

Federation endpoints are optional until Milestone 2. A node with federation
disabled returns `404` or an empty list with a capability flag; document which.

## 8. Admin endpoints

All admin endpoints require `Authorization: Bearer <token>`.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/api/v1/admin/queue` | Pending uploads |
| `POST` | `/api/v1/admin/content/{hash}/approve` | Publish quarantined content |
| `POST` | `/api/v1/admin/content/{hash}/reject` | Reject and delete the blob |
| `POST` | `/api/v1/admin/content/{hash}/remove` | Tombstone and stop serving |
| `POST` | `/api/v1/admin/content/{hash}/restrict` | Keep the row but stop public serving |
| `POST` | `/api/v1/admin/content/{hash}/reinstate` | Reverse a restriction or removal |
| `GET` | `/api/v1/admin/reports` | Open reports |
| `GET` | `/api/v1/admin/storage` | Blob/storage and orphan counters |
| `POST` | `/api/v1/admin/storage` | Delete orphan blobs |
| `POST` | `/api/v1/admin/content/{hash}/licence` | Set the local licence status |
| `GET` | `/api/v1/admin/stats` | Storage, queue and traffic counters |
| `GET` | `/api/v1/admin/blocks` | Local block rules |
| `POST` | `/api/v1/admin/blocks` | Add a URL/domain/tag block rule |
| `DELETE` | `/api/v1/admin/blocks/{id}` | Remove a block rule |

Mutating admin calls write an audit entry.

### `GET /api/openapi.json`

Serves the generated OpenAPI JSON for this node. The draft source is
[`openapi.yaml`](openapi.yaml); update both together.

## 9. Errors

```json
{
  "error": {
    "code": "validation_failed",
    "message": "The image has too many frames.",
    "detail": { "frames": 812, "max": 300 }
  }
}
```

`detail` is optional and must not leak filesystem paths, secrets or raw client
addresses. Suggested HTTP mappings:

| Code | HTTP |
|---|---|
| `bad_request`, `invalid_hash` | 400 |
| `unauthorized` | 401 |
| `forbidden`, `content_restricted` | 403 |
| `content_not_found` | 404 |
| `content_removed` | 410 |
| `payload_too_large` | 413 |
| `unsupported_media_type` | 415 |
| `validation_failed` | 422 |
| `rate_limited` | 429 |
| `conflict` | 409 |
| `service_unavailable` | 503 |

## 10. Rate limits

Responses may include:

```http
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 12
X-RateLimit-Reset: 1763088000
Retry-After: 30
```

Limits are per hashed client, endpoint class and node policy. Search and media
downloads may have separate budgets.

## 11. Caching

- Media bytes: immutable, one year, public.
- Manifests: short max-age (60 s) with `ETag`.
- Search/trending: no-store or a very short max-age.
- Admin: no-store.

## 12. Versioning

- `/api/v1` is the compatibility surface.
- Additive fields are allowed in v1; clients must ignore unknown fields.
- Removing or changing a field requires `/api/v2` and an ADR.
- The OpenAPI document is the source of truth for the implemented surface. Update
  it in the same change as the handler.

## 13. Open points

- Whether `/api/v1/media/{hash}` returns the manifest or a redirect.
- The exact anonymous upload quota and whether proof of work is mandatory.
- Whether reports accept a contact address at all (privacy).
- The node-list pagination shape once federation exists.
- The final media protocol name and version header.
