# ADR-003 — SHA-256 content identity and metadata separation

- Status: Accepted
- Date: 2026-11-14
- Affects: protocol, storage, API

## Context

Weft must let independent nodes identify the same media without trusting a URL,
and it must allow different nodes to disagree about metadata and licensing for the
same bytes. A URL is mutable and provider-specific; a hash is not.

## Decision

- Identify immutable bytes by SHA-256 over the original file: lowercase hex.
- Expose the identity as `urn:weft:sha256:<hash>`.
- Keep the bytes (`blobs`) separate from mutable local metadata (`content`).
- ActivityPub object IDs remain HTTPS URLs; the URN is a property
  (`weft:contentId`), not the ActivityPub `id`.
- The original bytes are never re-encoded; derivatives and thumbnails are
  separate artifacts.
- Different nodes may hold different metadata and licence claims for the same
  hash. A node must apply its own policy.

## Consequences

- Positive: deduplication, integrity verification and replica failover.
- Positive: metadata can be corrected or moderated without changing the hash.
- Negative: SHA-256 is not a proof of authorship or licensing.
- Negative: a node cannot change the bytes of an existing content ID; a new
  version needs a new hash.
- Follow-up: verify stored blobs on startup or first serve; add a repair tool.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| URL as identity | Mutable, provider-specific, link rot |
| BLAKE3 | Not requested; SHA-256 is widely available and sufficient |
| Hash including metadata | Metadata changes would change identity |
| Perceptual hash | Useful for duplicates but not exact integrity |

## References

- [`../PROTOCOL.md`](../PROTOCOL.md)
- [`../DATA_MODEL.md`](../DATA_MODEL.md)
- [`../CONTENT_LICENSING.md`](../CONTENT_LICENSING.md)
