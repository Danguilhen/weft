# Weft — threat model

Status: **design for review**. Initial model for the design phase. Revisit at
every milestone and after any protocol change. Implementation is not authorised
until [`DESIGN_REVIEW.md`](DESIGN_REVIEW.md) records approval.

This model covers the node, the media protocol and the federation surface. It
does not cover the security of independently operated nodes or client devices.

## 1. Method

- **STRIDE** per trust boundary: spoofing, tampering, repudiation, information
  disclosure, denial of service, elevation of privilege.
- **LINDDUN** for privacy: linking, identifying, non-repudiation, detecting,
  data disclosure, unawareness, non-compliance.
- **Abuse-case analysis** for anonymous uploads, licensing and moderation.
- Qualitative risk: impact × likelihood, with a residual-risk column.

Likelihood and impact are relative to a public, small self-hosted node. A
compromised decoder or signing key is treated as high impact even if unlikely.

## 2. Assets

| ID | Asset | Security property |
|---|---|---|
| A1 | Original media bytes | Integrity, availability |
| A2 | SHA-256 content identity | Integrity |
| A3 | Node signing keys | Confidentiality, integrity |
| A4 | Admin token | Confidentiality |
| A5 | PostgreSQL data | Integrity, confidentiality |
| A6 | Metadata, licence claims, moderation events | Integrity, authenticity |
| A7 | Client privacy (IP, JID, timing) | Confidentiality |
| A8 | Node availability and bandwidth | Availability |
| A9 | Disk and storage quota | Availability |
| A10 | Backups and key material | Confidentiality, integrity |

## 3. Actors and capabilities

| Actor | Capability |
|---|---|
| Anonymous visitor | Search, download, report; maybe upload |
| Contributor | Anonymous or pseudonymous uploader |
| Node operator | Full local control, admin token, moderation |
| Peer node | Federation, replication, receipts, reports |
| Malicious peer | Forged events, Sybil, fake availability, DoS |
| Malicious uploader | Malformed media, bombs, illegal content |
| Network observer | Traffic analysis, timing correlation |
| Compromised dependency | Code execution in build or runtime |
| Legal requester | Takedown, preservation, disclosure |

## 4. Trust boundaries and data flow

```text
[1] Internet client
        │ HTTPS
        ▼
[2] Reverse proxy ──▶ [3] weft-server ──▶ [4] PostgreSQL
                          │  │
                          │  └────────▶ [5] blob storage
                          │
                          ▼
                    [6] weft-decode worker
                          ▲
                          │ fd, no network
[7] Peer node ── AP ──▶ [8] inbox/signature ──▶ [3]
        │
        └── media pull ──▶ [9] outbound fetch ──▶ [5]
```

| Boundary | Trust | Main risks |
|---|---|---|
| 1 → 2 | Untrusted | HTTP abuse, body bombs, TLS misconfig |
| 2 → 3 | Trusted proxy only | Header spoofing if proxy trust is wrong |
| 3 → 4 | Trusted | SQL injection, credential leak, DB exposure |
| 3 → 5 | Trusted for storage | Path traversal, symlink, corrupt blob |
| 3 → 6 | Untrusted media | Decoder RCE, bombs, resource exhaustion |
| 7 → 8 | Untrusted | Forged signatures, replay, SSRF, floods |
| 3 → 9 | Outbound | SSRF, DNS rebinding, redirect abuse |

## 5. Key security invariants

1. Bytes are never served unless the stored blob matches the advertised hash.
2. Untrusted media is decoded only in the separate `weft-decode` worker with
   limits; never in the HTTP server process.
3. Remote JSON-LD contexts are never fetched. A pinned local context is used.
4. Media bytes never travel through ActivityPub.
5. No raw IP address, JID, email or contributor identity is stored in public
   metadata or propagated through federation.
6. No federation event is accepted without a valid signature, timestamp and
   replay check.
7. No central authority can force a node to accept, store or serve content.
8. Admin authentication never uses cookies.
9. Client filenames never determine storage paths.
10. The network cannot guarantee deletion of copies already downloaded; the
    documentation must say so.

## 6. Threat catalog

### 6.1 Upload and media

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| U-01 | Malformed GIF/image exploits the decoder and executes code | High | Medium | Separate worker, rlimits, no network/DB, sandbox, fuzzing, updates | 1 | Sandbox escape/zero-day remains |
| U-02 | Decompression bomb exhausts RAM/CPU/disk | High | High | File, pixel, frame, decoded-memory and time limits; stream to disk | 1 | Within-limits exhaustion |
| U-03 | Polyglot or parser differential (GIF/HTML/SVG) | High | Medium | Sniff + decode, allow-list, reject SVG/HTML, `nosniff`, no inline rendering | 1 | Novel format confusion |
| U-04 | EXIF/GPS or metadata leak | Medium | High | Strip derivatives; do not expose original metadata; document | 1 | Vendor-specific metadata |
| U-05 | Path traversal or symlink via filename | High | Medium | Hash-derived paths, `O_NOFOLLOW`, private temp, reject unexpected ownership | 1 | Local operator mistake |
| U-06 | Disk or quota exhaustion | High | High | Pre-check quotas, temp limits, per-client buckets, disk alerts | 1 | Distributed slow fill |
| U-07 | TOCTOU/hash mismatch between validation and serving | High | Low | Hash while streaming, verify before commit, atomic rename, scrub | 1 | Storage corruption |
| U-08 | Thumbnail decoder bomb | High | Medium | Same worker/limits for thumbnails; bounded size | 1 | Decoder bug |
| U-09 | Active content (SVG script, HTML) served inline | High | Medium | No SVG/HTML; `Content-Type` from decode; `nosniff`; `Content-Disposition` | 1 | Browser bug |
| U-10 | Duplicate uploads bypass quota or create races | Medium | Medium | Dedup by hash, idempotency key, transaction/upsert | 1 | Distributed races |
| U-11 | Temp file left readable or world-writable | Medium | Low | Private dir, 0600, cleanup on failure, tmpfs optional | 1 | Crash residue |
| U-12 | Dedup side channel reveals whether a hash exists | Low | High | Do not return existence to anonymous users beyond their upload; rate limit | 1 | Public content is public anyway |

### 6.2 API, auth and admin

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| A-01 | Admin token theft or brute force | High | Medium | 32-byte random token, Argon2id hash, rate limit, rotation, no logs | 1 | Host compromise |
| A-02 | CSRF on admin actions | High | Low | Bearer header, no cookie auth; strict CORS | 1 | Malicious extension |
| A-03 | SQL/FTS injection | High | Low | Parameterized queries, `websearch_to_tsquery`, no string SQL | 1 | ORM misuse |
| A-04 | XSS via title/tags/licence in admin UI | High | Medium | JSON API; clients escape; CSP if HTML admin added | 1/5 | Client bug |
| A-05 | SSRF via evidence URL fetch | High | Medium | HTTPS only, reject private/link-local, no redirects beyond limit | 5 | DNS rebinding |
| A-06 | Enumeration/pagination scraping | Medium | High | Rate limits, cursor caps, optional auth for bulk | 1 | Distributed scraping |
| A-07 | Cache poisoning via headers/host | High | Low | Fixed external URL, ignore Host for absolute URLs, proxy config | 1 | Proxy misconfig |
| A-08 | Timing/user enumeration | Low | Medium | Uniform errors, constant-time token check | 1 | Traffic analysis |
| A-09 | Permissive CORS | Medium | Medium | Disabled by default; explicit allow-list | 1 | Misconfiguration |
| A-10 | Error detail leaks paths/secrets | Medium | Medium | Central error mapping, redaction, no stack traces | 1 | Log leak |
| A-11 | Log injection via metadata | Low | Medium | Structured logging, escape newlines, length limits | 1 | Log parser bug |

### 6.3 Federation

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| F-01 | Forged activity or manifest | High | High | HTTP Signatures, canonical JSON, body digest, key verification | 2 | Key compromise |
| F-02 | Replay of a valid activity | High | Medium | Timestamp window, nonce/inbox dedupe, activity ID cache | 2 | Cache eviction window |
| F-03 | Key substitution or rotation attack | High | Medium | Key history, `validUntil`/revoked, pin after first verify, notify peers | 2 | First-contact MITM |
| F-04 | SSRF via actor, media or context URL | High | High | No remote contexts, HTTPS only, block private IPs, redirect/timeout limits | 2 | DNS rebinding |
| F-05 | JSON-LD/JSON bomb or deeply nested payload | High | Medium | Body size, depth/item limits, fixed context, streaming parser | 2 | Parser bug |
| F-06 | Inbox flood or amplification | High | High | Per-node/IP rate limits, body limits, backoff, quarantine | 2 | Distributed flood |
| F-07 | Malicious manifest with wrong hash/size | High | Medium | Verify signature, hash, size and limits before use | 2/3 | None after verification |
| F-08 | Unsigned or downgraded activity accepted | High | Medium | Require signatures for federation; no fallback | 2 | Misconfiguration |
| F-09 | Sybil nodes bypass block/local policy | Medium | High | Discovery ≠ trust; per-node policy; optional community lists | 2 | Sybil is unavoidable |
| F-10 | Spoofed moderation event | High | Medium | Signed by issuer; never auto-apply; local review | 2/5 | Issuer compromise |
| F-11 | Federation used to amplify traffic | High | Medium | Metadata only; pull media; quotas | 2/3 | Popular content demand |
| F-12 | Revoked key still trusted from cache | High | Medium | Bounded key cache, refresh on failure, revocation check | 2 | Offline peer |
| F-13 | Actor deletion vs key revocation ambiguity | Medium | Medium | Keep tombstone/actor history; document | 2 | Historical ambiguity |

### 6.4 Replication and accounting

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| R-01 | Peer forces storage or replication | High | High | Pull only; no push writes; local policy | 3 | None by design |
| R-02 | Replication bandwidth amplification | High | High | Quotas, concurrency, byte caps, 429/backoff | 3 | Within quota |
| R-03 | Bad hash/source poisoning | High | Medium | Verify SHA-256, mark unhealthy, retry another source | 3 | None after hash |
| R-04 | Quota bypass via ranges/retries | Medium | Medium | Count full transfer, cap retries, per-peer budget | 3 | Distributed abuse |
| R-05 | Fake availability / eclipse | Medium | High | Verify by fetch, diverse sources, no single trust | 3 | Partial view |
| R-06 | Receipt forgery or replay | High | Medium | Signatures, nonce, time window, countersignature | 4 | Collusion |
| R-07 | Self-dealing/collusion inflates score | Medium | High | Same-operator ignore, diminishing returns, bounded influence | 4 | Unavoidable Sybil |
| R-08 | Score gaming denies service to small nodes | Medium | Medium | Basic access never score-gated; bounded quotas | 4 | Policy error |
| R-09 | Unfair denial by local score | Medium | Medium | Transparent operator report; appeal/override | 4 | Operator choice |
| R-10 | Cache poisoning via replica URL | High | Medium | Verify hash, no redirects, HTTPS, source policy | 3 | DNS compromise |

### 6.5 Moderation and licensing

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| M-01 | False reports or report brigading | Medium | High | Rate limits, local review, no automatic global action | 1/5 | Operator load |
| M-02 | False licence claim presented as verified | High | High | Explicit statuses, evidence, never auto-verify, UI wording | 1 | Human error |
| M-03 | Takedown abuse or over-removal | Medium | Medium | Evidence, appeal/reinstate, local policy, legal review | 5 | Legal pressure |
| M-04 | Illegal content stored/federated for review | High | Medium | No sample federation, local restrict, specialist procedure | 5 | Operator error |
| M-05 | Malicious evidence URL/phishing | Medium | Medium | No auto-fetch to private networks; warn; sandbox fetch | 5 | User click |
| M-06 | Compromised moderator account | High | Medium | Strong token, rotate, audit, least privilege | 1/5 | Host compromise |
| M-07 | Report PII retained or leaked | Medium | Medium | Hashed contacts, retention, access control | 1 | Backup leak |
| M-08 | Moderation event treated as global truth | High | Medium | Events are inputs; local policy; documentation | 5 | Misinterpretation |

### 6.6 Privacy

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| P-01 | Client IP exposed to node/proxy | Medium | High | No third parties; short logs; document; user chooses trusted node | 1 | Node sees IP |
| P-02 | Timing correlation of upload/search/download | Medium | Medium | No analytics; short retention; document limits | 1 | Global observer |
| P-03 | XMPP JID/identity leaked to node | High | Medium | Client never sends JID; no accounts; tests | 6 | Client bug |
| P-04 | Logs/backups leak personal data | High | Medium | Redaction, encryption, retention, access control | 1 | Operator error |
| P-05 | Third-party embeds/tracking | High | Low | No embeds, no provider calls, no tracking params | 1 | Dependency |
| P-06 | Fingerprinting via metadata/errors | Medium | Low | Uniform responses, no device data | 1 | Traffic shape |
| P-07 | Anonymous upload deanonymized by IP retention | High | Medium | Salted rotating hash, no raw IP, short retention | 1 | Network observer |
| P-08 | Optional contributor key links history | Medium | Medium | Opt-in, separate key, warn, not federated | 5 | User choice |
| P-09 | Search queries logged or leaked | Medium | Medium | Do not log query strings; no federation | 1 | Proxy logs |
| P-10 | Federation metadata reveals private report details | High | Low | Never federate reports/contacts | 1/2 | Implementation bug |

### 6.7 Supply chain and operations

| ID | Threat | Impact | Likelihood | Mitigation | Phase | Residual |
|---|---|---|---|---|---|---|
| S-01 | Compromised dependency executes code | High | Medium | Lockfile, review, audit/deny, minimal deps, SBOM | 1+ | Zero-day |
| S-02 | Container/image tampering | High | Medium | Pin digests, sign images, minimal base, rebuild | later | Registry compromise |
| S-03 | CI secret leak | High | Medium | Least-privilege tokens, masked secrets, no fork secrets | 1 | CI compromise |
| S-04 | Backup theft | High | Medium | Encryption, access control, tested restore | 1 | Key custody |
| S-05 | Database exposed to network | High | Medium | Localhost default, firewall, strong role, TLS if remote | 1 | Misconfig |
| S-06 | Node key file theft | High | Medium | 0600, root-only, secret manager option, rotation | 1 | Host compromise |
| S-07 | Insecure defaults | High | Medium | Secure config defaults, startup validation | 1 | Operator override |
| S-08 | Migration failure/data loss | High | Medium | Forward-only, backup, restore drill | 1 | Operator error |
| S-09 | Disk exhaustion by logs/quarantine | Medium | High | Rotation, quotas, alerts, cleanup | 1 | Burst |
| S-10 | Log injection/poisoning | Low | Medium | Structured logs, escaping, size limits | 1 | Parser bug |

## 7. Abuse cases

### Anonymous upload flood

An attacker uploads thousands of small files from many addresses. Controls:
review mode, per-client token bucket on a salted hash, burst limit, storage
quota, quarantine expiry, optional proof of work. The node must remain able to
serve existing content while the queue fills. Alert on queue depth and disk.

### Popular GIF traffic spike

A GIF is posted to a large chat. The origin could serve every request. Controls:
immutable caching, replica advertisement, demand-driven replication, per-node
egress caps. The origin must not be required for correctness; if no replica
exists, it degrades to slower service, not failure.

### Forged moderation event

A malicious node claims content is illegal or its licence invalid. Controls:
signature verification, local policy, evidence review, no automatic removal.
A compromised node can still persuade careless operators; documentation and UI
must distinguish an allegation from a decision.

### Fake receipts

Two colluding nodes exchange receipts for content nobody requested to inflate
scores. Controls: countersignatures, nonces, same-operator detection,
diminishing returns, bounded influence. Residual: Sybil collusion is not fully
solvable; scores are resource signals, not trust.

### Copyright laundering

An uploader labels copyrighted media as CC0. Controls: explicit unverified
status, evidence, disputes, local acceptance policy, no "verified" claim. The
node cannot determine copyright automatically.

### Illegal content

An uploader posts illegal material. Controls: immediate local restriction,
specialist reporting, no federation of samples, minimal evidence preservation,
legal advice. Never build a shared repository of such material.

## 8. Security requirements

| ID | Requirement | Level | Phase |
|---|---|---|---|
| SR-01 | Decode untrusted media only in `weft-decode` with rlimits and a timeout | Must | 1 |
| SR-02 | Enforce file, pixel, frame, memory and time limits before serving | Must | 1 |
| SR-03 | Verify SHA-256 before committing and before serving a repaired blob | Must | 1 |
| SR-04 | Never derive storage paths from client filenames | Must | 1 |
| SR-05 | Never fetch remote JSON-LD contexts | Must | 2 |
| SR-06 | Require signatures and replay protection for federation events | Must | 2 |
| SR-07 | Reject media bytes in ActivityPub activities | Must | 2 |
| SR-08 | Pull replication only; never write on a peer's request | Must | 3 |
| SR-09 | Countersign transfer receipts; never trust an unsigned receipt | Must | 4 |
| SR-10 | Apply local policy to every remote moderation event | Must | 5 |
| SR-11 | Store no raw IPs or JIDs; use salted hashes with retention | Must | 1 |
| SR-12 | Admin auth uses bearer tokens, never cookies | Must | 1 |
| SR-13 | CORS disabled by default; explicit allow-list only | Must | 1 |
| SR-14 | `nosniff` and safe content types on media; no SVG/HTML | Must | 1 |
| SR-15 | Central error mapping; no paths, secrets or stack traces | Must | 1 |
| SR-16 | Transactional outbox for federation; persistent inbox dedupe | Must | 2 |
| SR-17 | Backup encryption and a tested restore procedure | Should | 1 |
| SR-18 | Fuzz upload decoding, manifests and receipts | Should | 1/2 |
| SR-19 | Sign release artifacts and scan dependencies | Should | later |
| SR-20 | Document anonymity limits and legal responsibilities | Must | 1 |

## 9. Residual risks

| Risk | Why it remains | Handling |
|---|---|---|
| Decoder zero-day/sandbox escape | Any decoder has bugs | Review mode, sandbox, fast updates, minimal exposure |
| Global network observer | Timing/size correlation | Document; no claim of strong anonymity |
| Sybil/collusion in accounting | No global identity | Bounded scores; basic access not gated |
| Deletion after download | Decentralisation | Document; tombstones and notifications only |
| Compromised peer key | Signatures only prove key possession | Revocation, pinning, operator review |
| Below-profile node | Operators run below 2 GB | `weft doctor`, unsupported warning, documented limits |
| Legal obligations | Software cannot provide legal compliance | Operator guide and legal review |
| Generic ActivityPub interop | Ed25519/RSA gap | Target Weft-aware nodes; document |

## 10. Out of scope

- Security of client devices, browsers or the XMPP server.
- Malicious node operators with legal authority.
- Physical access to the host.
- Side-channel attacks on Ed25519/SHA-256 implementations.
- A global anonymity guarantee.
- Content accuracy beyond the stated licence model.

## 11. Review checklist

- [ ] Every entry point is authenticated or intentionally public.
- [ ] Every untrusted input has size, time and memory limits.
- [ ] Every federation input has signature, replay and policy checks.
- [ ] Every remote fetch is SSRF-safe and HTTPS-only.
- [ ] Every state change is transactional and auditable.
- [ ] Every error path returns a safe error shape.
- [ ] Logs and metrics contain no IPs, JIDs, queries, tokens or bodies.
- [ ] The threat model matches the architecture's trust boundaries.
- [ ] New threats have an owner, mitigation, phase and residual risk.
- [ ] The design review is updated and approved.

## 12. Open security questions

- Q3: Ed25519-only vs RSA compatibility for ActivityPub.
- Q4/Q7: upload policy and anti-abuse defaults.
- Q11: minimum node size and decode budget.
- Q12: legal jurisdiction and takedown procedure.
- Q13: moderation-event transport.
- Whether the MVP can require the decode worker or must allow in-process decode
  for development only.
- Whether receipts should be public or operator-only.
