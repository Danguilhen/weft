# ADR-010 — Default upload policy is review

- Status: Accepted
- Date: 2026-11-14
- Affects: API, moderation, operations, privacy

## Context

Weft accepts anonymous contributions without accounts. A new public node has no
reputation or moderation history. Fully open uploads expose the operator to
spam, malware and illegal content; a closed node cannot receive contributions.
The policy must be configurable per node because a private node has different
needs.

## Decision

- The default public policy is **`review`**: uploads are validated, hashed and
  stored in quarantine; they are not searchable or publicly retrievable until an
  operator approves them.
- `open` is an explicit opt-in. The node logs a startup warning and applies the
  anti-abuse limits from ADR-011.
- `closed` is available for private or read-only nodes.
- A review queue with approval, rejection, expiry and a global size cap is part
  of the MVP.
- Rejected uploads delete the blob when no other content references it.
- The uploader is told “pending review”, never given a public URL for quarantined
  content.

## Consequences

- Positive: a new public node is not immediately flooded or legally exposed.
- Positive: the same code supports a private node with `closed`.
- Negative: publication is delayed by operator review.
- Negative: the operator must run the queue; the MVP needs good queue tooling.
- Follow-up: measure queue depth and expiry on a public test node.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| `open` by default | Too risky for an unknown public node |
| `closed` by default | Blocks the contribution model |
| Accounts/invite only | Useful as a local option, not a network default |
| Post-hoc moderation only | Illegal content may be public before review |

## References

- [`ADR-005`](ADR-005-anonymous-uploads.md)
- [`ADR-011`](ADR-011-anti-abuse.md)
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md) §6.1, §7
- [`../CONFIG.md`](../CONFIG.md)
