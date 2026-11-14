# ADR-001 — ActivityPub metadata plus a media distribution protocol

- Status: Accepted
- Date: 2026-11-14
- Affects: protocol, federation, operations

## Context

Weft must federate node discovery, content announcements, tags, licence claims
and moderation events without making the origin node serve every download.
ActivityPub is a good fit for signed social objects and inbox delivery, but it is
not designed to distribute large media files efficiently. Using it for bytes
would create amplification, retry storms and unbounded inbox bodies.

## Decision

Use two layers:

1. **Metadata and events:** ActivityPub actor, inbox/outbox and signed activities
   for discovery, content metadata, reports and moderation/licence events.
2. **Media distribution:** a dedicated HTTPS protocol for manifests, bytes,
   replicas and receipts.

ActivityPub activities may link to a manifest or media URL. They must never carry
the media bytes themselves. The media protocol is documented in
[`../PROTOCOL.md`](../PROTOCOL.md); the federation layer in
[`../FEDERATION.md`](../FEDERATION.md).

The server must work with federation disabled; the REST and media APIs remain
usable standalone.

## Consequences

- Positive: small inbox messages, cacheable media, clear security boundaries,
  independent layers that can evolve separately.
- Positive: a node can use ActivityPub for discovery without accepting media from
  every peer.
- Negative: two protocols to specify and test.
- Negative: generic ActivityPub servers will not understand the media layer.
- Follow-up: define the JSON-LD namespace, signature profile and media protocol
  versioning.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| ActivityPub for bytes too | Amplification, retries and large inbox bodies |
| Central media CDN | Violates the no-central-authority principle |
| IPFS/libp2p mandatory | Operational complexity for small nodes; not required |
| Plain HTTPS index with no federation | No discovery or moderation propagation |

## References

- [`../ARCHITECTURE.md`](../ARCHITECTURE.md)
- [`../PROTOCOL.md`](../PROTOCOL.md)
- [`../FEDERATION.md`](../FEDERATION.md)
- ADR-008
