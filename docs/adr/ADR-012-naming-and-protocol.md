# ADR-012 — Project naming and the Weft Media Protocol

- Status: Accepted
- Date: 2026-11-14
- Affects: protocol, API, crates, documentation

## Context

The provisional name “OpenGIF” is already used by unrelated projects and a
website. The project chose **Weft** as the public name. The media protocol was
provisionally called the “Weft Media Distribution Protocol”, abbreviated **WMD**,
which is an unfortunate acronym. The name and protocol version appear in wire formats, crate
names and documentation, so they should be fixed before implementation and
federation.

## Decision

- Public project name: **Weft**.
- Crate names: `weft-core`, `weft-client`, `weft-server`, `weft-db`,
  `weft-storage`, `weft-validation`, `weft-federation`, `weft-cli`.
- Content identity remains `urn:weft:sha256:<hash>`.
- The media protocol is the **Weft Media Protocol (WMP)**.
- Protocol version string: `weft-wmp/0.1`.
- The JSON-LD namespace is resolved by ADR-014:
  `https://w3id.org/weft-network/ns#`. A brand domain is still optional; do not use a
  domain the project does not control.
- Check crates.io and the relevant trademark/domain registries before the first
  public release; this ADR records intent, not a legal clearance.

## Consequences

- Positive: a distinct name and a non-embarrassing protocol acronym.
- Positive: wire identifiers are fixed before any implementation.
- Negative: any existing drafts using the old `weft-mdp/0.1` string must be updated.
- Negative: the w3id redirect must be registered and hosted before federation.
- Follow-up: register the w3id redirect, publish the JSON-LD context, and
  reserve crate names.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep OpenGIF | Name collision; weak identity |
| Keep WMD | Unfortunate acronym |
| Change the content URN per domain | Unnecessary coupling and migration cost |
| Defer naming | Wire formats would need a breaking change later |

## References

- [`../PROTOCOL.md`](../PROTOCOL.md)
- [`../OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) Q1/Q2/Q14
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md)
