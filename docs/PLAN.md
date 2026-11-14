# Weft — open work

Status: living backlog, started 2026-11-14.

This file contains **open work only**. Do not append progress notes; update
[`STATUS.md`](STATUS.md) for state and delete a bullet when it is done.

Scope and principles: [`../README.md`](../README.md). Decisions:
[`adr/README.md`](adr/README.md). Undecided choices:
[`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md).

## 1. Release gate

The first public node release requires:

- The standalone MVP demo in [`MVP.md`](MVP.md) passes on a clean host.
- Upload validation has limits, fuzz coverage and no known bypass.
- No endpoint returns untrusted bytes without correct `Content-Type` and hash.
- Admin authentication, rate limits and log redaction are verified.
- The licence model never presents an unverified claim as verified.
- A backup/restore and upgrade have been exercised.
- `cargo fmt`, `cargo clippy`, `cargo test`, the docs check and the container
  build are green.
- `docs/STATUS.md`, `docs/API.md`, `docs/openapi.yaml` and `docs/CONFIG.md`
  match the release.
- At least two independent operators have run a node and reviewed the design.

Federation, replication and accounting are **not** required for the first
standalone release.

## 2. Milestone 0 — Foundation

Complete. Evidence is in [`STATUS.md`](STATUS.md); the next open phase is
Milestone 1.

## 3. Milestone 1 — Standalone MVP

**Gate:** `DESIGN_REVIEW.md` is approved for Milestone 1; the first CI run on
`main` passes all jobs, including container and Compose; DR-01 (decoder worker),
DR-02 (pinned context) and DR-03 (outbox/inbox schema) are accepted as
implementation requirements.

Scope and acceptance: [`MVP.md`](MVP.md).

### Remaining

- Add a search-index trait before replacing PostgreSQL; keep the current FTS
  query as the default.
- Add licence-evidence verification actions and tests for the licence state
  machine beyond the initial claim.
- Add a duplicate/idempotent upload test and tests for dispute/reject.
- Add orphan deletion and storage quota enforcement on top of the storage report.
- Verify the first CI run, the container build and the Compose health check.
- Run the acceptance demo on the supported 2 vCPU / 2 GB / 20 GB profile and
  record the numbers.

Exit criteria: the ten-step acceptance demo in `MVP.md` passes on the supported
2 vCPU / 2 GB / 20 GB node with PostgreSQL.

## 4. Milestone 2 — ActivityPub federation

**Gate:** Q2 and Q3 are resolved in ADRs; DR-06 golden vectors and DR-09
removal flow are implemented and tested; a two-node interop plan exists.

- Add the node actor, Ed25519 key publication, key rotation and revocation
  history.
- Pin a local JSON-LD context and reject or ignore remote context URLs.
- Implement HTTP Signatures for inbound and outbound requests, with replay
  protection, clock skew, body digests and a nonce cache.
- Add the transactional outbox and persistent inbox dedupe tables/workers.
- Implement `/.well-known/weft`, the node descriptor and node discovery via
  bootstrap nodes and peer exchange.
- Implement signed `Create`, `Update` and `Delete` for content metadata; never
  attach media bytes.
- Implement `Flag` reports and the custom moderation-event objects described in
  `FEDERATION.md`.
- Add per-node allow/block lists and an inbox rate limit.
- Add a two-node integration test: publish, discover, fetch metadata, update and
  delete.
- Add a federation smoke test that runs two containers with distinct keys.
- Document Ed25519/RSA interoperability in an ADR before claiming fediverse
  compatibility.

Exit criteria: two independent nodes discover each other, exchange metadata,
verify every signature, and apply independent moderation decisions; neither
requires a central service.

## 5. Milestone 3 — Media replication

- Track local demand per content and per peer without retaining IP addresses.
- Add replica advertisements and a verified replica list per content.
- Implement pull replication: select a source, stream the blob, verify SHA-256,
  store, advertise.
- Implement per-node storage, file-size, bandwidth and concurrency quotas.
- Implement LRU/LFU eviction with pinning for locally uploaded content.
- Add cache expiry and a dry-run eviction report.
- Add replica health checks and source removal for unavailable replicas.
- Add tests for partial downloads, hash mismatch, source failover and quota
  refusal.
- Add a three-node Docker test with one node offline.

Exit criteria: a popular item is served from a replica while the origin is
offline; an oversized or corrupt transfer is rejected without corrupting the
cache; a small node can opt out of replication entirely.

## 6. Milestone 4 — Cooperative resource accounting (experimental)

- Define signed transfer receipts and their canonical encoding.
- Implement receipt issuance by the serving node and countersignature by the
  requester.
- Implement local scoring from verified bandwidth, stored replicas and
  availability.
- Add diminishing returns for repeated transfers of the same content and peer.
- Add quotas and fair-use limits driven by the local score; basic access must
  never depend on a high score.
- Add manipulation checks: forged receipts, replay, self-dealing and collusion;
  document their limits.
- Add metrics and an operator report explaining why a quota was applied.
- Keep the feature disabled by default and label it experimental in the API and
  docs.

Exit criteria: two nodes exchange receipts, each calculates its own score, and a
small node with no contribution still browses and downloads within fair use. No
global ledger exists.

## 7. Milestone 5 — Licence and moderation operations

- Add licence evidence capture (URL plus fetched hash), verification workflow and
  dispute states.
- Add signed licence-dispute and moderation events over ActivityPub.
- Add removal and reinstatement notifications; stop replication for restricted
  content.
- Add local and opt-in community blocklists; never a mandatory global list.
- Add illegal-content reporting and immediate local restriction procedures that
  do not federate samples.
- Add an operator moderation CLI and a minimal admin HTML page if the API alone
  proves insufficient.
- Add privacy-preserving contributor pseudonyms, optional and never required.
- Document the legal limitations and the jurisdiction review needed before public
  deployment.
- Add tests for state transitions: claimed → evidence → verified/disputed →
  invalidated/reinstated.

Exit criteria: a disputed item is restricted locally, known replicas are
notified, and all nodes apply their own policy. The node never asserts legal
verification it did not perform.

## 8. Milestone 6 — Jabu integration

### 6a. Animated attachments (independent, small)

- In Jabu, choose the original URL for animated MIME types (`image/gif`,
  animated WebP, APNG) instead of the static thumbnail.
- Add a GIF badge and avoid cropping tall media; keep the thumbnail as a
  placeholder until the original loads.
- Add unit tests for the display-URL choice.
- Keep this change independent of the Weft network so it can ship first.

### 6b. Weft client and picker

- Add `weft-client` as a path/git dependency of Jabu during development.
- Define a `GifProvider` trait in Jabu so Weft is the first implementation, not
  a hard-coded assumption.
- Add settings: node URL, optional custom node, rating, autoplay, send mode.
- Add a GIF button next to emoji and stickers with search, trending, categories
  and a preview grid.
- Show attribution and licence state on every result and in the preview.
- Implement loading, empty, rate-limited and offline states.
- Send via XEP-0363 by default: fetch the selected asset, upload it, send a file
  message with the original dimensions and attribution. Keep a link mode behind
  a setting.
- Add no tracking parameters and do not expose the user's JID to the node.
- Add tests with a mock provider and a fake upload slot.

Exit criteria: a Jabu user searches a Weft node, previews a GIF, sends it to a
1:1 chat and a room, and the recipient can retrieve it after the Weft node is
offline (because XEP-0363 stored it). The animation fix is verified separately.

## 9. Milestone 7 — Network testing

- Run at least three independent nodes with different storage and bandwidth
  budgets.
- Simulate a popular item, a traffic spike and the origin going offline.
- Simulate a small node that refuses replication and one that accepts it.
- Test malicious uploads, decompression bombs, oversized metadata and active
  content.
- Test forged federation events, replayed signatures, fake receipts and fake
  availability advertisements.
- Test a removal and a licence dispute across the three nodes.
- Test a node with no public IPv4 and a node behind a reverse proxy.
- Publish the results, adjust defaults and record protocol changes in ADRs.

Exit criteria: the network degrades predictably, no node can force another to
store data, and the operator guide matches observed behaviour.

## 10. Cross-cutting work

- Keep `docs/STATUS.md` current after every behaviour change.
- Keep `docs/openapi.yaml`, `docs/schemas/` and `docs/PROTOCOL.md` in sync.
- Add fuzz targets for upload decoding, manifests and receipts.
- Add dependency and container scanning once CI builds images.
- Add SBOM and signed releases once there is something to release.
- Revisit `OPEN_QUESTIONS.md` at every milestone boundary.
- Update the threat model after each phase, not only at the end.

## 11. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Decoder attack surface | Node compromise or DoS | Limits, isolated worker, fuzzing, allow-list |
| Anonymous upload abuse | Spam, illegal content | Quarantine default, rate limits, local policy |
| Federation trust mistakes | Poisoned metadata | Signatures, per-node allow/block, independent policies |
| Replication traffic | Bandwidth saturation | Pull-only, quotas, eviction, opt-in |
| Accounting gaming | Unfair quotas | Local scoring, diminishing returns, bounded influence |
| Legal exposure | Operator liability | Licence honesty, takedown procedure, legal review |
| Scope creep | No MVP | Phase gates; MVP excludes federation and accounting |
| Small-node performance | Operators leave | Postgres-first search, no transcoding by default, hard limits |

## 12. Task selection rules

1. Work on Milestone 0 before Milestone 1, and Milestone 1 before federation.
2. A task blocked by an open question stays blocked until the question is
   resolved in an ADR.
3. Do not start a later milestone because it is more interesting; record the
   reason in `OPEN_QUESTIONS.md` if the order must change.
4. One task per pull request, with the docs and tests it affects.
5. Remove a bullet here when it is done; record the evidence in `STATUS.md`.
