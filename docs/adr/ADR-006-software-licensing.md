# ADR-006 — AGPL server and MIT core/client

- Status: Accepted
- Date: 2026-11-14
- Affects: licensing, contribution, client reuse

## Context

Weft is a network service that should remain open even when hosted by a third
party. At the same time, clients such as Jabu must be able to embed a client
library and domain types without copyleft obligations. A single licence cannot
serve both goals well.

## Decision

- Server-side crates (`weft-server`, `weft-db`, `weft-storage`,
  `weft-validation`, `weft-federation`, `weft-cli`): **AGPL-3.0-or-later**.
- Client-safe crates (`weft-core`, `weft-client`): **MIT**.
- Every source file carries an SPDX header.
- Do not copy AGPL server code into the MIT crates. Shared code belongs in
  `weft-core` under MIT.
- Contributions are accepted under the licence of the crate they touch; no CLA
  and no copyright assignment.

## Consequences

- Positive: hosted modifications to the server must be shared.
- Positive: Jabu can depend on `weft-client` without becoming AGPL.
- Negative: two licence files and careful crate boundaries.
- Negative: some users avoid AGPL; that is accepted for the server.
- Follow-up: add a licence check to CI once the workspace exists.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| All AGPL | Blocks permissive client reuse |
| All MIT | Allows a proprietary hosted fork of the server |
| MPL server | Weaker network copyleft than AGPL |
| Dual AGPL/commercial | No commercial arm is planned |

## References

- [`../../LICENSING.md`](../../LICENSING.md)
- [`../../LICENSE`](../../LICENSE)
- [`../../LICENSE-MIT`](../../LICENSE-MIT)
