# ADR-009 — Supported node profile: 2 vCPU, 2 GB, 20 GB

- Status: Accepted
- Date: 2026-11-14
- Affects: operations, configuration, MVP, federation

## Context

The brief suggested a 1 vCPU / 1 GB / 10 GB node. The measured budget for
PostgreSQL, the Rust server, the decode worker and the OS leaves little headroom
at 1 GB, especially with a 256 MB decoder. A 1 GB profile would force lower
quality limits for every operator and make OOM failures likely under a small
burst.

The node size is an operational profile, not a protocol claim. Peers must not
trust advertised capacity; the design verifies bytes, observations and receipts.

## Decision

- The supported and recommended public node is **2 vCPU, 2 GB RAM, 20 GB
  storage**.
- Default limits for that profile:
  - `upload.max_decoded_memory = 256MB`
  - `upload.max_pixels = 16000000`
  - `upload.max_frames = 300`
  - `max_concurrent_decodes = 2`
  - `database.max_connections = 10`
  - `storage.max_size = 20GB`
- `weft doctor` reads the local cgroup/available memory and disk and warns or
  refuses to start when the configured limits exceed the machine.
- A 1 GB node is **unsupported** for public uploads. The binary may run there,
  but it is not a tested or documented deployment.
- Federation and Meilisearch are not part of the 2 GB profile; they are optional
  additions with their own budget.

## Consequences

- Positive: defaults are safe and testable; the MVP demo has a clear target.
- Positive: no false promise that a 1 GB VPS is enough for a public node.
- Negative: a small hobby VPS below 2 GB is excluded from support.
- Negative: operators with more RAM still need to raise limits explicitly.
- Follow-up: measure the demo on the reference profile and record the numbers in
  `STATUS.md`.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| 1 vCPU / 1 GB / 10 GB | Too tight with PostgreSQL and a safe decoder |
| Two profiles (minimal + standard) | Doubles the support and test matrix |
| 4 GB recommended | Excludes reasonable small VPS plans |
| No documented profile | Defaults become arbitrary and support impossible |

## References

- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) §17
- [`../CONFIG.md`](../CONFIG.md)
- [`../MVP.md`](../MVP.md)
- [`../OPERATIONS.md`](../OPERATIONS.md)
