# Weft — open questions

These choices are deliberately unresolved. Do not silently decide one in code or
in an ADR without updating this file. Each question records the options, the
impact and the current default.

**Status values:** `blocking` (stops a milestone), `before-release` (must be
settled before the first public node), `later` (can wait).

| # | Question | Options | Impact | Status |
|---|---|---|---|---|
| Q1 | Public name, domain, crate namespace | Weft (selected); another name | Crate names, URNs, JSON-LD namespace, domain ownership | resolved by ADR-012 (name/crates/URN; domain in Q2) |
| Q2 | JSON-LD namespace URI | `https://weft.network/ns#`; a registered w3id; another domain | Federation compatibility; must be stable before Phase 2 | blocking for federation |
| Q3 | ActivityPub signature interoperability | Ed25519 only; Ed25519 plus RSA-2048 compatibility key | Mastodon-style interop vs simpler implementation | blocking for federation |
| Q4 | Default upload policy | `review` (quarantine); `open`; `closed` | Abuse exposure vs contribution friction | resolved by ADR-010 |
| Q5 | Search backend | PostgreSQL FTS; Meilisearch from day one | Operational weight, search quality | resolved by ADR-002; revisit after MVP |
| Q6 | Jabu send mode | Re-upload via XEP-0363; send Weft URL; both | Privacy, link rot, storage, OMEMO behaviour | resolved by ADR-007 (upload default; link opt-in) |
| Q7 | Anonymous-upload anti-abuse | Per-IP token bucket; optional proof of work; both | CPU, spam resistance, small-node cost | resolved by ADR-011 |
| Q8 | Operator identity for receipt de-duplication | Optional operator ID; no operator ID | Accounting manipulation resistance vs anonymity | later |
| Q9 | Data retention defaults | None; 7 days; 30 days for abuse logs | Legal exposure, privacy | before-release |
| Q10 | Documentation licence | CC BY-SA 4.0; CC BY 4.0; no doc licence | Reuse and translation | before-release |
| Q11 | Minimum supported node size | 2 vCPU / 2 GB / 20 GB (selected); smaller is unsupported | Defaults, search and decode budgets | resolved by ADR-009 |
| Q12 | Legal review jurisdiction | UK; EU; multi-jurisdiction | Takedown and illegal-content procedures | before-release |
| Q13 | Public moderation-event transport | ActivityPub `Flag` plus custom events; custom only | Interop and clarity | before-release |
| Q14 | Name of the media protocol | WMP; another name | Documentation only | resolved by ADR-012 (WMP; `weft-wmp/0.1`) |

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
