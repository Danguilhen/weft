# ADR-005 — Anonymous uploads with a configurable review queue

- Status: Accepted
- Date: 2026-11-14
- Affects: API, moderation, privacy, operations

## Context

Weft promises anonymous contributions without accounts. Open anonymous uploads
also attract spam, malware and illegal content, and a new node has no reputation
to filter them. Requiring accounts or a central approval authority would violate
the project principles.

## Decision

- Browsing, search and download never require an account.
- Upload policy is per node: `open`, `review` or `closed`.
- The default for a public node is `review`: uploads are validated, hashed and
  quarantined until an operator approves them.
- Anonymous uploads are rate-limited by a salted, rotating hash of the client
  address; raw addresses are not stored.
- Optional proof of work may supplement rate limits; it is not required by
  default.
- Optional pseudonymous contributor keys are allowed later but are never
  mandatory and are not federated by default.
- The node may keep private abuse evidence for the shortest period allowed by
  law.

## Consequences

- Positive: no account barrier; a new public node is not immediately flooded.
- Positive: the operator controls the risk without a central authority.
- Negative: a review queue needs operator time; a quiet node may delay
  publication.
- Negative: proof of work costs CPU and may disadvantage low-power clients.
- Follow-up: measure queue load on a public test node and tune defaults in
  `CONFIG.md`.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Fully open by default | Too risky for a new public node |
| Accounts required | Violates anonymous contribution |
| Central approval service | Violates no-central-authority |
| Invite-only | Useful as a local option, not a network default |

## References

- [`../PRIVACY.md`](../PRIVACY.md)
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md)
- [`../CONFIG.md`](../CONFIG.md)
