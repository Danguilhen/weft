# Weft Media Distribution Protocol (WMDP)

Version: `weft-mdp/0.1` (draft).

This protocol distributes immutable media bytes, metadata manifests, replica
advertisements and signed transfer receipts between Weft nodes. It is separate
from ActivityPub: ActivityPub announces and describes content, while this
protocol delivers it.

## 1. Conventions

- HTTPS only. Nodes behind a reverse proxy may terminate TLS.
- Base URL: the node's canonical public origin, e.g. `https://gifs.example`.
- Times: RFC 3339 UTC.
- Hashes: lowercase SHA-256 hex.
- Signatures: Ed25519 over RFC 8785 (JCS) canonical JSON, encoded as base64url
  without padding.
- Canonical signing input: the JSON object with its `signature` field removed,
  encoded with JCS. Do not sign a pretty-printed or reordered document.
- Content identity: `urn:weft:sha256:<hash>`.
- Media type parameter: `application/weft+json` for signed JSON documents where
  practical; `application/json` is accepted for readability.

## 2. Node descriptor

`GET /.well-known/weft` returns a signed descriptor. Caching is allowed for a
short period (60 seconds) but discovery must not be cached indefinitely.

```json
{
  "version": "weft-node/0.1",
  "nodeId": "https://gifs.example/actor",
  "name": "Example GIF node",
  "description": "A small community node",
  "publicUrl": "https://gifs.example",
  "actor": "https://gifs.example/actor",
  "keys": [
    {
      "keyId": "https://gifs.example/actor#key-1",
      "alg": "Ed25519",
      "publicKey": "base64url-public-key",
      "createdAt": "2026-11-14T00:00:00Z",
      "validUntil": null,
      "revokedAt": null
    }
  ],
  "capabilities": {
    "api": ["v1"],
    "media": ["weft-mdp/0.1"],
    "federation": "activitypub",
    "replication": true,
    "search": "postgres"
  },
  "limits": {
    "maxUploadBytes": 15728640,
    "maxPixels": 16000000,
    "maxFrames": 300,
    "maxReplicaBytes": 10737418240
  },
  "policyUrl": "https://gifs.example/policy",
  "contact": "https://gifs.example/contact",
  "updatedAt": "2026-11-14T00:00:00Z",
  "signature": {
    "alg": "Ed25519",
    "keyId": "https://gifs.example/actor#key-1",
    "value": "base64url-signature"
  }
}
```

Requirements:

- `nodeId` is stable across key rotation. The actor URL is the node identity.
- Keys may rotate; old keys remain listed with `validUntil`/`revokedAt` so
  previously signed documents remain verifiable.
- A node must not treat discovery as trust. Policy is separate.

## 3. Media bytes

### `GET /media/{hash}`

Returns the original bytes for an active content record.

Headers:

```http
Content-Type: image/gif
Content-Length: 1234567
ETag: "sha256:3a7bd3e2360a..."
Digest: sha-256=:base64-digest:
Cache-Control: public, max-age=31536000, immutable
Accept-Ranges: bytes
```

- `hash` must be 64 lowercase hex characters.
- The server must verify the stored blob against the hash before first serve or
  at startup; it must not serve a blob whose hash does not match.
- Range requests return `206` with `Content-Range`; invalid ranges return `416`.
- `HEAD` is supported and returns the same headers without a body.
- Active content returns `200`; quarantined/restricted content returns `404` to
  the public; removed content returns `410` with no body and a tombstone
  reference in headers if the node chooses.
- `Content-Disposition` may suggest a filename but must not enable script
  execution. `X-Content-Type-Options: nosniff` is required.

### `GET /media/{hash}/manifest`

Returns the signed manifest for the content. It is the authoritative local
metadata record and may differ from another node's manifest for the same bytes.

```json
{
  "version": "weft-manifest/0.1",
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "hash": "3a7bd3e2360a...",
  "mime": "image/gif",
  "size": 1234567,
  "width": 480,
  "height": 270,
  "frames": 42,
  "durationMs": 3200,
  "createdAt": "2026-11-14T00:00:00Z",
  "updatedAt": "2026-11-14T00:00:00Z",
  "origin": "https://gifs.example/actor",
  "sources": [
    {
      "nodeId": "https://gifs.example/actor",
      "url": "https://gifs.example/media/3a7bd3e2360a...",
      "verifiedAt": "2026-11-14T00:00:00Z",
      "bytes": 1234567
    }
  ],
  "licence": {
    "claim": "CC0-1.0",
    "spdx": "CC0-1.0",
    "url": "https://creativecommons.org/publicdomain/zero/1.0/",
    "status": "unverified",
    "evidence": [
      {
        "url": "https://example.org/original",
        "note": "Original creator's post",
        "fetchedAt": null
      }
    ]
  },
  "moderation": {
    "state": "active",
    "updatedAt": "2026-11-14T00:00:00Z",
    "eventsUrl": "https://gifs.example/api/v1/media/3a7bd3e2360a.../events"
  },
  "tags": ["cat", "reaction"],
  "signature": {
    "alg": "Ed25519",
    "keyId": "https://gifs.example/actor#key-1",
    "value": "base64url-signature"
  }
}
```

`size` is the exact original byte length. `width`, `height`, `frames` and
`durationMs` are optional when unknown but must be present for GIFs processed by
the MVP. `licence.status` is one of:

- `unverified`
- `evidence_provided`
- `verified_evidence`
- `disputed`
- `invalidated`

A manifest must never present `unverified` or `evidence_provided` as legally
verified. The signature covers every field except `signature`.

### `GET /media/{hash}/sources`

```json
{
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "sources": [
    {
      "nodeId": "https://gifs.example/actor",
      "url": "https://gifs.example/media/3a7bd3e2360a...",
      "verifiedAt": "2026-11-14T00:00:00Z",
      "bytes": 1234567
    }
  ],
  "signature": {
    "alg": "Ed25519",
    "keyId": "https://gifs.example/actor#key-1",
    "value": "base64url-signature"
  }
}
```

Sources are hints, not a trust list. A consumer must verify the hash of anything
it downloads.

### Manifest caching and removal

- A manifest response has a strong `ETag` derived from the canonical JSON of the
  signed manifest and `Cache-Control: public, max-age=60`.
- `If-None-Match` returns `304` when the manifest has not changed.
- Removing content stops bytes (`410`) and marks the manifest
  `moderation.state = removed`; the tombstone remains for audit.
- The removing node may publish a signed `weft:ModerationEvent` of category
  `CONTENT_REMOVED` and notify known replicas. Each replica applies its own
  policy; a notification is not a command.
- A node must not serve bytes from a replica once it has locally removed the
  content, even if the origin still has it.

## 4. Replication

Replication is pull-only. A node must never be able to force another node to
download or store content.

### 4.1 Selecting a source

A replication worker:

1. reads a manifest or a replica advertisement;
2. filters sources by policy, health and byte limits;
3. requests the blob with a range-friendly GET;
4. verifies length and SHA-256;
5. writes to a temporary path and atomically promotes it;
6. records itself as a replica and, optionally, advertises it.

A source returning a bad hash is marked unhealthy and its advertisement is not
trusted again until it is re-verified.

### 4.2 Advertisements

`POST /api/v1/replicas` (node-to-node, signed):

```json
{
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "nodeId": "https://replica.example/actor",
  "url": "https://replica.example/media/3a7bd3e2360a...",
  "bytes": 1234567,
  "verifiedAt": "2026-11-14T00:00:00Z",
  "signature": {
    "alg": "Ed25519",
    "keyId": "https://replica.example/actor#key-1",
    "value": "base64url-signature"
  }
}
```

The receiving node may accept, ignore or rate-limit the advertisement. An
advertisement is not proof that the bytes are available; the receiver verifies
by fetching when it needs them.

### 4.3 Transfer receipts

A receipt records that a serving node transferred bytes to a requesting node. It
is a local accounting input, never a global credit.

```json
{
  "receiptId": "01J...",
  "contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "bytes": 1234567,
  "servedBy": "https://gifs.example/actor",
  "servedTo": "https://client.example/actor",
  "startedAt": "2026-11-14T00:00:00Z",
  "endedAt": "2026-11-14T00:00:04Z",
  "nonce": "base64url-random-16-bytes",
  "signature": {
    "alg": "Ed25519",
    "keyId": "https://gifs.example/actor#key-1",
    "value": "base64url-signature"
  }
}
```

The requester countersigns with its own key and stores the pair. Receipts are
exchanged over `POST /api/v1/receipts` or an ActivityPub event. Both sides keep
the receipt; no third party is trusted to aggregate them.

Anti-manipulation rules (experimental, Phase 4):

- receipts expire if not countersigned within a configured window;
- a nonce is single-use per server;
- repeated transfers of the same content to the same peer have diminishing
  weight;
- transfers between nodes that declare the same operator are not counted as
  independent contributions;
- a node may ignore any receipt it cannot verify.

## 5. Signed requests between nodes

Signed node-to-node requests carry:

```http
X-Weft-Node: https://gifs.example/actor
X-Weft-Key-Id: https://gifs.example/actor#key-1
X-Weft-Timestamp: 2026-11-14T00:00:00Z
X-Weft-Nonce: base64url-16-bytes
X-Weft-Signature: base64url-signature
Digest: sha-256=:base64-digest:
```

The signature input is:

```text
METHOD
/path?query
X-Weft-Timestamp
X-Weft-Nonce
SHA-256(body)
```

or, for ActivityPub, the HTTP Signatures profile described in
[`FEDERATION.md`](FEDERATION.md). Reject timestamps outside a five-minute window,
reused nonces, mismatched body digests and unknown or revoked keys.

## 6. Errors

All JSON errors use:

```json
{
  "error": {
    "code": "content_not_found",
    "message": "No active content exists for this hash."
  }
}
```

Suggested codes: `bad_request`, `invalid_hash`, `content_not_found`,
`content_removed`, `content_restricted`, `payload_too_large`,
`unsupported_media_type`, `validation_failed`, `rate_limited`, `unauthorized`,
`forbidden`, `conflict`, `service_unavailable`, `signature_invalid`,
`replay_detected`.

## 7. Caching and privacy

- Media is immutable; `Cache-Control: public, max-age=31536000, immutable` is
  correct for active content.
- Manifests may change; use `ETag` and short max-age (60 seconds).
- Do not add tracking query parameters, cookies or client IDs.
- Do not log raw IP addresses, full user agents or query strings that could
  identify a person.
- Nodes must not follow arbitrary redirects when fetching media; allow only
  HTTPS, reject private/link-local addresses and limit redirect depth.

## 8. Versioning

- Protocol version is in the descriptor and manifests.
- Additive fields are allowed in a minor version; consumers must ignore unknown
  fields.
- Removing or reinterpreting a field requires a new version and an ADR.
- A node that does not understand a version returns `415` rather than guessing.

## 9. Test vectors

The implementation must ship golden vectors for:

- canonical JSON of a manifest and a receipt;
- Ed25519 signatures over those bytes;
- a manifest with a rotated key;
- a rejected manifest with a mismatched hash;
- a replayed nonce and an expired timestamp;
- a replica advertisement that fails byte verification.

These vectors are part of the acceptance criteria for Phase 2 and must not be
generated by the implementation under test alone.

## 10. Open points

- Rate-limit headers and exact quotas.
- Whether manifests are cached by content hash plus update sequence.
- The receipt exchange transport (HTTP only, ActivityPub only, or both).
- The response when a blob exists but is under moderation review.
- The final media protocol name. See [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md).
