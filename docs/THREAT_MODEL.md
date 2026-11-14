# Weft — threat model

Status: initial model for the design phase. Revisit at every milestone boundary.

## 1. Assets

- Availability and integrity of the node and its storage.
- The original media bytes and their SHA-256 identity.
- Node signing keys and admin credentials.
- User privacy: no IP addresses, JIDs, contributor identities or browsing history
  leaked to peers or logs.
- The integrity of metadata, licence claims and moderation events.
- The reputation and legal exposure of node operators.

## 2. Actors

| Actor | Capability |
|---|---|
| Anonymous visitor | Search, download, report, maybe upload |
| Contributor | Anonymous or pseudonymous uploader |
| Node operator | Full local control, admin token, moderation |
| Peer node | Federation, replication, receipts, reports |
| Malicious peer | Forged events, Sybil nodes, fake receipts, DoS |
| Malicious uploader | Malformed media, decompression bombs, illegal content |
| Network observer | Traffic analysis, timing correlation |
| Compromised dependency | Supply-chain code execution |

## 3. Trust boundaries

1. Internet → public API.
2. Upload bytes → decoder worker.
3. Remote activity → inbox and signature verification.
4. Remote manifest → local index and replication decision.
5. Admin API → local database and storage.
6. Database/storage → application integrity checks.

The decoder and the inbox are the highest-risk boundaries.

## 4. Threats and mitigations

### 4.1 Upload and media

| Threat | Mitigation | Phase |
|---|---|---|
| Malformed image exploits decoder | Allow-list, resource-limited worker, fuzzing, dependency updates | 1 |
| Decompression bomb | Pixel, frame, decoded-memory and time limits | 1 |
| Polyglot or active content | Sniff format, reject SVG/HTML, re-derive MIME, `nosniff` | 1 |
| Path traversal | Hash-derived paths only; never use client filenames | 1 |
| Storage exhaustion | Quotas, per-file limit, per-client buckets, disk alarms | 1 |
| Metadata leak (GPS/EXIF) | Strip metadata from derivatives; do not expose originals' metadata in API | 1 |
| Thumbnail parser attack | Decode in the same sandbox; treat thumbnails as untrusted | 1 |
| Hash collision confusion | SHA-256 identity; verify bytes before serving | 1 |

### 4.2 Abuse and content

| Threat | Mitigation | Phase |
|---|---|---|
| Spam uploads | Quarantine default, per-client limits, optional proof of work | 1 |
| Illegal content | Immediate local restriction, reporting procedure, no sample federation | 5 |
| Copyright fraud | Unverified status, evidence, disputes, local policy | 1/5 |
| Tag spam / SEO abuse | Normalisation, length limits, operator policy | 1 |
| Report brigading | Rate limits, local review, no automatic global action | 1/5 |
| Impersonation of a node | Signatures, key pinning after first verification, operator contact | 2 |

### 4.3 Federation

| Threat | Mitigation | Phase |
|---|---|---|
| Forged activity | HTTP Signatures, canonical JSON, body digest, key lookup | 2 |
| Replay | Timestamp window, nonce cache, activity ID dedupe | 2 |
| SSRF via actor/media URL | HTTPS only, resolve and reject private/link-local, redirect limits | 2 |
| Inbox flood | Body limits, rate limits, per-node quotas, backoff | 2 |
| Poisoned metadata | Treat remote metadata as untrusted; verify signatures; local policy | 2 |
| Malicious manifest | Signature and hash verification; never auto-download from inbox | 2 |
| Key compromise | Rotation, revocation, historical verification, operator notice | 2 |
| Sybil peers | Don't grant trust on discovery; per-node policy; bounded reputation | 2/4 |
| Fake replica advertisement | Verify by fetching; mark unhealthy on mismatch | 3 |
| Forced storage | Pull-only; never accept a push that writes to storage | 3 |
| Bandwidth exhaustion | Quotas, concurrency limits, egress caps, 429/backoff | 3 |
| Fake receipts | Countersignatures, nonce, time window, local scoring, diminishing returns | 4 |
| Collusion / self-dealing | Optional operator IDs, ignore same-operator transfers, bounded influence | 4 |
| Global blocklist capture | Blocklists opt-in and local; no mandatory authority | 5 |

### 4.4 Operations

| Threat | Mitigation | Phase |
|---|---|---|
| Leaked admin token | Env/secret file, hashed at rest, rotation, audit log | 1 |
| Database exposed | Bind localhost, strong role, TLS if remote, firewall | 1 |
| Log leakage | Redaction, no bodies/tokens/IPs, retention limits | 1 |
| Backup theft | Encrypt backups, restrict access, test restore | 1 |
| Dependency compromise | Lockfile, review, `cargo deny`/audit, SBOM | 1+ |
| Supply-chain image tampering | Pin images, sign releases, minimal images | later |

## 5. Privacy threats

- **IP exposure.** Any direct request reveals the client address to the node and
  its reverse proxy. Do not add third-party embeds, analytics or provider calls.
- **Timing correlation.** A network observer can correlate an upload with a
  search or a chat message. Anonymity is not guaranteed against a global
  observer.
- **Contributor linkage.** Optional pseudonymous keys can link contributions over
  time. They must be optional, separate from node keys, and never propagated by
  default.
- **Federated metadata.** Node announcements, manifests and moderation events are
  public. Do not include private reports or contact details.
- **Retention.** Hashed client identifiers must expire; logs must be short-lived.

## 6. Illegal content

This is separate from copyright disputes.

- Do not federate, replicate or store samples for review.
- Restrict locally and immediately on credible notice.
- Preserve only the minimum evidence required by law.
- Follow specialist reporting procedures and obtain legal advice.
- Do not build a shared repository of illegal material.

## 7. Residual risks

- A determined attacker can still exhaust resources within any finite quota.
- Decentralisation cannot guarantee deletion of copies already downloaded.
- Anonymity is bounded by network observation and operator logs.
- Reputation and receipts are signals, not proof of honest behaviour.
- A compromised node key can sign malicious events until it is revoked; peers may
  have cached old keys.
- Generic ActivityPub implementations may not verify Ed25519 signatures; the
  first federation milestone targets Weft-aware nodes.

## 8. Review checklist

At each milestone:

- Has a new trust boundary appeared?
- Are all new inputs size-, time- and memory-limited?
- Are new logs free of personal data and secrets?
- Does a new federation event have a signature, replay protection and policy?
- Does a new endpoint have auth, rate limits and an error shape?
- Is there a test for the failure path, not only the happy path?
