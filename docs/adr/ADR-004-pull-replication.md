# ADR-004 — Pull-based replication and signed receipts

- Status: Accepted
- Date: 2026-11-14
- Affects: protocol, storage, accounting

## Context

Popular content must not be served only by its origin, but a node must never be
able to force another node to store data. Push replication makes resource
exhaustion and amplification trivial. Cooperative accounting also needs evidence
that a transfer happened without creating a global ledger or a cryptocurrency.

## Decision

- Replication is **pull-based**. A node discovers a manifest or replica
  advertisement, decides locally, fetches the bytes, verifies SHA-256 and stores
  them.
- A replica advertisement is a signed hint, not a command and not proof of
  availability.
- The serving node issues a signed receipt for a completed transfer; the requester
  countersigns it and both keep a copy.
- Scoring is **local**. There is no global reputation database, no transferable
  credit and no consensus ledger.
- Repeated transfers of the same content to the same peer have diminishing
  weight; transfers between nodes declaring the same operator do not count as
  independent contributions.
- Accounting is experimental and disabled by default.

## Consequences

- Positive: no forced storage, bounded amplification, natural caching/failover.
- Positive: receipts are useful evidence without a central authority.
- Negative: pull replication needs demand tracking and source health checks.
- Negative: receipts can be forged or colluded; scores are bounded signals, not
  proof.
- Follow-up: define the exact scoring formula and quotas after the network test
  in Milestone 7.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Push replication | Forced storage and amplification |
| Global ledger/token | Violates the project principles |
| Fixed replication factor | Wastes resources on small nodes |
| No accounting | Contributors have no signal to size quotas |

## References

- [`../PROTOCOL.md`](../PROTOCOL.md)
- [`../PLAN.md`](../PLAN.md) Milestone 3–4
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md)
