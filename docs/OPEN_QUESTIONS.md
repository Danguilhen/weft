# Weft — open questions

These choices are deliberately unresolved. Do not silently decide one in code or
in an ADR without updating this file. Each question records the options, the
impact and the current default.

**Status values:** `blocking` (stops a milestone), `before-release` (must be
settled before the first public node), `later` (can wait).

| # | Question | Options | Impact | Status |
|---|---|---|---|---|
| Q1 | Public name, domain, crate namespace | Weft; another name | Crate names, URNs, JSON-LD namespace, domain ownership | before-release |
| Q2 | JSON-LD namespace URI | `https://weft.network/ns#`; a registered w3id; another domain | Federation compatibility; must be stable before Phase 2 | blocking for federation |
| Q3 | ActivityPub signature interoperability | Ed25519 only; Ed25519 plus RSA-2048 compatibility key | Mastodon-style interop vs simpler implementation | before-release |
| Q4 | Default upload policy | `review` (quarantine); `open`; `closed` | Abuse exposure vs contribution friction | blocking for MVP |
| Q5 | Search backend | PostgreSQL FTS; Meilisearch from day one | Operational weight, search quality | resolved by ADR-002; revisit after MVP |
| Q6 | Jabu send mode | Re-upload via XEP-0363; send Weft URL; both | Privacy, link rot, storage, OMEMO behaviour | blocking for integration |
| Q7 | Anonymous-upload anti-abuse | Per-IP token bucket; optional proof of work; both | CPU, spam resistance, small-node cost | blocking for MVP |
| Q8 | Operator identity for receipt de-duplication | Optional operator ID; no operator ID | Accounting manipulation resistance vs anonymity | later |
| Q9 | Data retention defaults | None; 7 days; 30 days for abuse logs | Legal exposure, privacy | before-release |
| Q10 | Documentation licence | CC BY-SA 4.0; CC BY 4.0; no doc licence | Reuse and translation | before-release |
| Q11 | Minimum supported node size | 1 vCPU / 1 GB / 10 GB; 2 vCPU / 2 GB / 50 GB | Defaults, search and decode budgets | blocking for MVP |
| Q12 | Legal review jurisdiction | UK; EU; multi-jurisdiction | Takedown and illegal-content procedures | before-release |
| Q13 | Public moderation-event transport | ActivityPub `Flag` plus custom events; custom only | Interop and clarity | before-release |
| Q14 | Name of the media protocol | Weft Media Distribution (WMD); another name | Documentation only | later |

## How to resolve

1. Open a discussion or issue with the question number.
2. Record the chosen answer in an ADR (or modify the owning ADR).
3. Update this table: mark it `resolved`, link the ADR, and keep the history in
   the ADR.
4. Update any affected docs and schemas in the same change.

## Assumptions currently used by the design

These are not decisions; they are assumptions that must be re-checked:

- SHA-256 is sufficient for content identity; no collision-resistant alternative
  is required for the MVP.
- Ed25519 signatures over RFC 8785 JSON are sufficient for node manifests and
  receipts.
- ActivityPub nodes are Weft-aware; full Mastodon interop is not required for the
  first federation milestone.
- A node can run without object storage by using a local filesystem.
- A GIF can be accepted without re-encoding; derivatives are optional.
- The first client (Jabu) can upload the selected GIF through XEP-0363.
