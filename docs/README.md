# Weft documentation

Weft is documented before it is implemented. These documents are the
specification for the first milestones, not a description of finished code.

## Reading order

1. [`../README.md`](../README.md) — what Weft is and why.
2. [`STATUS.md`](STATUS.md) — what exists today.
3. [`PLAN.md`](PLAN.md) — the open-work queue and phase gates.
4. [`DESIGN_REVIEW.md`](DESIGN_REVIEW.md) — review findings and the implementation gate.
5. [`ARCHITECTURE.md`](ARCHITECTURE.md) — components and data flow.
6. [`MVP.md`](MVP.md) — the first standalone milestone.
7. [`PROTOCOL.md`](PROTOCOL.md) — Weft Media Protocol (WMP).
8. [`FEDERATION.md`](FEDERATION.md) — ActivityPub integration.
9. [`API.md`](API.md) and [`openapi.yaml`](openapi.yaml) — REST surface.
10. [`DATA_MODEL.md`](DATA_MODEL.md) — PostgreSQL model.
11. [`CONFIG.md`](CONFIG.md) — node configuration.
12. [`THREAT_MODEL.md`](THREAT_MODEL.md), [`PRIVACY.md`](PRIVACY.md) — security and privacy.
13. [`CONTENT_LICENSING.md`](CONTENT_LICENSING.md) — media licence model.
14. [`OPERATIONS.md`](OPERATIONS.md) — self-hosting and maintenance.
15. [`XMPP.md`](XMPP.md) — Jabu integration.
16. [`GOVERNANCE.md`](GOVERNANCE.md) — project decision-making.
17. [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) — unresolved choices.
18. [`context/weft.jsonld`](context/weft.jsonld) — pinned JSON-LD context.
19. [`adr/README.md`](adr/README.md) — decision records.

## Ownership

| Document | Owns |
|---|---|
| `STATUS.md` | What exists, what is tested, what is open |
| `PLAN.md` | Open work only |
| `DESIGN_REVIEW.md` | Architecture/threat-model review and implementation gate |
| `ARCHITECTURE.md` | Components, boundaries, data flow |
| `PROTOCOL.md` | Media wire protocol |
| `FEDERATION.md` | ActivityPub objects, signatures, events |
| `API.md` + `openapi.yaml` | REST surface |
| `DATA_MODEL.md` | PostgreSQL schema and lifecycle |
| `CONFIG.md` | Configuration keys and defaults |
| `THREAT_MODEL.md` | Adversaries and mitigations |
| `PRIVACY.md` | Data collection, retention, anonymity limits |
| `CONTENT_LICENSING.md` | Media licence claims, evidence and disputes |
| `OPERATIONS.md` | Deployment, backups, upgrades, monitoring |
| `XMPP.md` | Client integration |
| `GOVERNANCE.md` | Project and maintainer process |
| `OPEN_QUESTIONS.md` | Undecided choices |
| `context/` | Pinned JSON-LD context and namespace notes |
| `adr/` | Accepted decisions and their history |
| `schemas/` | Machine-readable wire schemas; see [`schemas/README.md`](schemas/README.md) |

If two documents disagree, the ADR wins over the design document, and
`STATUS.md` wins over any claim about what is implemented. Fix the conflict in
the owning document instead of adding a third description.

## Conventions

- Protocol examples use JSON with comments only where marked illustrative.
- Times are RFC 3339 in UTC.
- Hashes are lowercase hexadecimal SHA-256 unless a document says otherwise.
- WMP manifest signatures are Ed25519 over RFC 8785 canonical JSON, base64url
  without padding. ActivityPub uses RSA-2048 HTTP Signatures (ADR-013).
- API paths are relative to the node origin and start with `/api/v1`.
- `urn:weft:sha256:<hash>` identifies immutable bytes.
- The public name is Weft; the JSON-LD namespace is fixed at
  `https://w3id.org/weft/ns#` (ADR-014). A brand domain remains optional.