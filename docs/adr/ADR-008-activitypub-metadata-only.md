# ADR-008 — ActivityPub announces metadata, never media bytes

- Status: Accepted
- Date: 2026-11-14
- Affects: federation, protocol, security

## Context

ActivityPub inboxes are not a content delivery network. Sending large media
through inboxes causes amplification, retry storms, storage pressure and privacy
leaks. Weft still needs a standard, signed way to discover and describe content.

## Decision

- ActivityPub carries `Document` objects that reference a media URL and a signed
  manifest; it never carries the bytes.
- Content objects include standard fields (`name`, `summary`, `mediaType`, `url`,
  `digest`) and the `weft:*` extension for content ID, dimensions, tags, licence
  and moderation references.
- `Create`, `Update`, `Delete`, `Flag` and `weft:ModerationEvent` are the
  federation activities.
- A receiver must not download bytes merely because an inbox activity arrived.
  Media retrieval is a separate, policy-controlled action.
- A node may index metadata without replicating bytes.

## Consequences

- Positive: small inbox messages and predictable federation traffic.
- Positive: media can be cached and replicated independently.
- Negative: two fetches (metadata and media) for a full sync.
- Negative: a generic ActivityPub server cannot render the media itself.
- Follow-up: define the exact JSON-LD context and object schema before Phase 2.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| `attachment` with bytes in the activity | Inbox amplification and size limits |
| Only a URL with no manifest | No signature or integrity metadata |
| A custom federation protocol only | Loses ActivityPub discovery and tooling |

## References

- [`../FEDERATION.md`](../FEDERATION.md)
- [`../PROTOCOL.md`](../PROTOCOL.md)
- ADR-001
