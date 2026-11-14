# Weft — design review: architecture and threat model

Review date: 2026-11-14
Documents: [`ARCHITECTURE.md`](ARCHITECTURE.md),
[`THREAT_MODEL.md`](THREAT_MODEL.md)
Scope: design for the standalone MVP, federation, replication and accounting.
Reviewer: architecture author, for maintainer approval.
Verdict: **approved for Milestone 1 (standalone MVP), subject to CI verification.
Federation, replication and accounting remain gated.**

## 1. Purpose

This review checks that the architecture and threat model are coherent,
implementable on a small node, consistent with the brief and safe enough to
start. It records what changed, what remains open and what must be approved
before code is written.

## 2. Summary

The design is coherent and the two-layer split is sound: ActivityPub carries
metadata and events, and a separate HTTPS protocol carries immutable bytes. The
MVP is achievable as a standalone node; federation and accounting remain
properly gated behind later phases.

The first drafts under-specified four things that matter for security and
operations. They are now fixed or explicitly gated:

1. **Decoder isolation** is now a concrete `weft-decode` worker with file
   descriptor passing, rlimits, a timeout and no network/database access.
2. **Federation input handling** now pins a local JSON-LD context and forbids
   remote context fetching, closing a major SSRF/DoS path.
3. **Federation delivery durability** now uses a transactional outbox and a
   persistent inbox dedupe table.
4. **Small-node sizing** is resolved: the supported public profile is 2 vCPU /
   2 GB / 20 GB; below that is unsupported and `weft doctor` warns or refuses.

Threat coverage is broad, including upload decoders, federation replay, SSRF,
replication quotas, receipt gaming, licence fraud, privacy and supply chain. The
remaining risks are mostly inherent to a decentralised, anonymous network and
are documented rather than hidden.

## 3. Findings

| ID | Severity | Area | Finding | Resolution / status |
|---|---|---|---|---|
| DR-01 | High | Upload | The first draft named a "validation worker" but did not define the process boundary, IPC or limits. A thread in the web process would not be a security boundary. | **Addressed in architecture.** `weft-decode` is a separate process, fd/path handoff, rlimits, timeout, no network/DB; sandbox recommended. Must be verified in Milestone 1. |
| DR-02 | High | Federation | ActivityPub JSON-LD can trigger remote context fetches, creating SSRF and DoS paths. | **Addressed.** Pin a local context; never fetch remote contexts; validate a fixed JSON shape; HTTPS-only outbound with private-IP rejection. |
| DR-03 | High | Federation | Publishing an event and delivering it are not atomic; a crash can lose announcements. | **Addressed.** Transactional outbox plus persistent inbox dedupe; both tables are now in `DATA_MODEL.md`. Verify the transaction/publisher behaviour in Phase 2. |
| DR-04 | High | Operations | The brief's "1 GB / 1 vCPU" target with PostgreSQL and a 256 MB decode budget is unrealistic for a public node. | **Resolved by ADR-009.** Supported profile is 2 vCPU / 2 GB / 20 GB; below is unsupported. |
| DR-05 | Medium | Federation | Ed25519-only ActivityPub signatures and the final JSON-LD namespace are unresolved. Generic fediverse interoperability may fail. | **Open (Q2, Q3).** Federation is blocked until decided. MVP is unaffected; target Weft-aware nodes initially. |
| DR-06 | Medium | Protocol | Manifest caching and signature test vectors are not fully specified. A mutable manifest needs an ETag/version rule. | **Addressed.** Manifest ETag/`If-None-Match` semantics and the golden-vector requirement are now in `PROTOCOL.md`; the vectors are a Phase 2 acceptance test. |
| DR-07 | Medium | Abuse | The default anonymous upload policy and anti-abuse mechanism are still open. | **Resolved by ADR-010 and ADR-011.** `review` default; token buckets/quotas; optional PoW. |
| DR-08 | Medium | API | Full-text search must not become SQL injection and queries must not leak to logs or peers. | **Addressed.** Parameterized `websearch_to_tsquery`, no query logging, no federation of queries. Verify with tests. |
| DR-09 | Medium | Moderation | Tombstone and removal propagation is described but has no concrete event flow for replicas. | **Addressed.** Removal returns `410`, the manifest records `removed`, and a signed `CONTENT_REMOVED` event can notify replicas; each replica applies local policy. Test this in Phase 5. |
| DR-10 | Medium | Replication | Demand tracking, source selection and accounting are not testable at design level. | **Accepted phase gate.** Implement and test in Phases 3–4; do not expose accounting before the network test. |
| DR-11 | Low | Admin/API | Admin auth, CSRF and CORS defaults were not explicit. | **Addressed.** Bearer tokens, no cookies, CORS disabled by default, loopback/allow-list, central error mapping. |
| DR-12 | Low | Project | The public name, domain and JSON-LD namespace are not settled; "OpenGIF" is already used elsewhere. | **Partially resolved by ADR-012.** Weft/WMP names chosen; domain and namespace remain Q2. |
| DR-13 | Low | Scope | The product is called a GIF network but the MVP also accepts static PNG/JPEG/WebP. | **Clarified.** Media is "GIF and animated media"; static images are allowed as attachments for the MVP and can be restricted by config. |
| DR-14 | Low | Client | Jabu send mode and the animated-attachment fix are separate concerns. | **Resolved by ADR-007.** XEP-0363 upload is the default; link mode is opt-in with no silent fallback. |

## 4. Architecture review

### Strengths

- The two-layer split keeps inboxes small and media cacheable.
- Content addressing gives deduplication, integrity and replica failover.
- Pull-only replication cannot force storage; quotas and eviction are local.
- Discovery is explicitly separated from trust.
- Anonymous uploads are paired with a review policy and local moderation.
- The decode worker, transactional outbox and pinned context close the three
  most dangerous design gaps.
- The MVP boundary is realistic: no federation, no replication, no accounting.

### Concerns

- Two protocols increase the specification and test surface. The MVP must not
  accidentally depend on federation.
- PostgreSQL-first search is deliberately modest; adding Meilisearch later must
  not change the API.
- The accounting phase is the most speculative. Keep it experimental and
  bounded; basic access must never depend on a score.
- A single-node deployment still has a single point of failure. That is
  acceptable for small communities; replication is what spreads availability.
- The supported profile is 2 GB after ADR-009; below that is unsupported, not a supported "minimal" mode.

### Feasibility

- Milestone 0 and the standalone MVP are feasible in Rust with Axum, SQLx and
  PostgreSQL. The main work is the upload pipeline and moderation state machine,
  not the HTTP surface.
- Federation is feasible for Weft-aware nodes, but not for generic ActivityPub
  servers without an RSA/key-profile decision.
- Replication is feasible; accounting should be built only after the network
  test provides real traffic data.

## 5. Threat model review

### Coverage

The model covers:

- the upload/decoder boundary, storage paths and hash integrity;
- API injection, auth, CORS, SSRF and error leakage;
- federation signatures, replay, JSON-LD, inbox floods and key rotation;
- replication storage/bandwidth abuse, source poisoning and receipts;
- licence fraud, report brigading and illegal content;
- privacy: IPs, timing, JIDs, logs, queries and contributor keys;
- supply chain, backups, database exposure and migrations.

### Top five risks

1. **Decoder compromise (U-01/U-02).** Mitigated by isolation and limits, not
   eliminated. Review mode and fast updates are part of the control.
2. **Federation SSRF/replay (F-02/F-04).** Mitigated by pinned contexts,
   signatures, replay cache and outbound network policy.
3. **Anonymous upload abuse (U-06, M-01).** Mitigated by review, quotas and
   rate limits; the operator still carries the load.
4. **Accounting collusion (R-06/R-07).** Cannot be solved fully; bounded scores
   and basic access floors are the design response.
5. **Legal exposure (M-02/M-04).** The software can only make compliance
   possible; the operator needs a takedown procedure and legal advice.

### Gaps accepted for later

- Client-side rendering of GIFs and untrusted metadata is the client's
  responsibility; the API returns data, not HTML.
- Evidence fetching needs the same SSRF protections as federation; it is
  scheduled for Phase 5.
- Backup encryption tooling is an operations task; the design requires it, the
  implementation is later.
- No strong anonymity against a global observer; this is documented as a
  limitation, not a promise.

## 6. Consistency with the brief

| Brief requirement | Where handled | Status |
|---|---|---|
| No central authority, no blockchain | Principles, architecture, governance | Covered |
| Anonymous contributions and privacy | ADR-005, privacy model, threat model | Covered |
| Content identity and metadata separation | ADR-003, data model | Covered |
| Two-layer federation and media | ADR-001/008, architecture | Covered |
| Demand-driven replication | ADR-004, protocol, plan | Covered (Phase 3) |
| Cooperative accounting | Protocol, plan | Experimental (Phase 4) |
| Upload validation and abuse | MVP, threat model | Covered |
| Licence claims, evidence, disputes | Content licensing | Covered |
| Moderation events and tombstones | Federation, content licensing | Covered; test in Phase 5 |
| ActivityPub integration | Federation | Covered, blocked on Q2/Q3 |
| REST API | API + OpenAPI | Draft covered |
| XMPP/Gabu integration | XMPP doc, ADR-007 | Covered (Phase 6) |
| Deliverables | Plan, docs index | Covered |

## 7. Conditions for authorisation

### Before Milestone 0 (repository scaffolding only)

- Maintainer approves this review and the architecture/threat model.
- Q11 is resolved by ADR-009; no further action is required.

Milestone 0 approval covers the workspace, CI, config loader, migrations
skeleton, Docker Compose and health endpoints. It does not cover upload,
federation or replication. That scope has been completed and recorded in
`STATUS.md`; it must not be extended without a new approval.

### Before Milestone 1 (standalone MVP)

- Q4 (ADR-010) and Q7 (ADR-011) are accepted; the limits in ADR-009 are used.
- DR-01 (decoder worker) and DR-02 (pinned context) are accepted as
  implementation requirements.
- DR-03 outbox/inbox tables are recorded for the Phase 2 migration; the MVP does not need to send federation events.
- The upload acceptance criteria and limit defaults are confirmed.

### Before Phase 2 (federation)

- Q2 and Q3 are answered in ADRs; Q1/Q14 are resolved by ADR-012.
- DR-06 manifest ETag/version and signature golden vectors are implemented.
- DR-09 removal-event flow is exercised in the two-node test.
- A two-node test plan exists.

### Before Phase 4 (accounting)

- Phase 3 replication has real traffic data.
- The scoring formula, diminishing returns and quota floors are specified and
  reviewed; accounting remains opt-in.

## 8. Verdict

**Approved for Milestone 1 (standalone MVP). Federation, replication and
accounting remain gated.**

The architecture and threat model are internally consistent and materially
safer than the first draft after the four high-severity changes. The remaining
blockers are the maintainer sign-off and, for federation only, Q2/Q3 (namespace
and key profile). The MVP decisions are recorded.

Milestone 1 is approved and in progress. The design review verification gate is
the first CI run, including the container and Compose jobs. Federation,
replication and accounting remain gated.

## 9. Sign-off

| Role | Name | Decision | Date |
|---|---|---|---|
| Maintainer | — | Approved Milestone 0 (scaffold only) | 2026-11-14 |
| Maintainer | — | Approved Milestone 1 (standalone MVP) | 2026-11-14 |
| Architecture author | — | Submitted for review | 2026-11-14 |
| Security reviewer | — | Pending | — |

Milestone 0 and Milestone 1 approvals are recorded. Do not extend approval to
federation, replication or accounting without a new sign-off.
