# Weft — status

Status: Milestone 1 in progress; the standalone vertical slice works. Started
2026-11-14. This is the only status page.

## Summary

| Area | State | Evidence |
|---|---|---|
| Design documentation | Complete for Phase 1 | This docs pack and `PLAN.md` |
| Architecture and threat model | Complete through Milestone 1; review recorded | `ARCHITECTURE.md`, `THREAT_MODEL.md` |
| Design review | Approved for Milestones 0 and 1; CI verification pending | `DESIGN_REVIEW.md` |
| Implementation | Milestone 1 in progress; vertical slice works | `DESIGN_REVIEW.md` §7–9 |
| Documentation link check | Passing | `python3 tools/check_docs.py` |
| Rust tests | 32 passing | `cargo test --workspace` |
| PostgreSQL smoke test | Passing | `cargo test -p weft-server --test postgres_smoke -- --ignored` |
| Rust workspace | Milestone 0 scaffold complete | `Cargo.toml` |
| Nix flake/dev shell | Passing | `nix develop`, `nix flake check`, `nix build .#weft` |
| Standalone node MVP | Complete locally; CI/acceptance verification pending | `upload_flow` test |
| ActivityPub federation | Not started | — |
| Media replication | Not started | — |
| Contribution accounting | Not started | — |
| Licence/moderation workflow | Partial: queue/actions/reports/disputes | `weft-server` routes |
| Jabu GIF picker | Not started | — |
| Jabu animated-attachment fix | Not started | Tracked in `PLAN.md` |

The standalone vertical slice is working: upload → decode → quarantine/review →
approval → search → signed manifest → media retrieval, with admin reports and
moderation actions. Federation, replication, accounting, local block rules and
the full operational metrics/OpenAPI surface are not implemented yet. Do not
claim otherwise in README, issues or release notes.

## What exists

- `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSING.md`.
- Architecture, protocol, federation, API, data-model, config, MVP, threat
  model, design review, privacy, operations, content-licensing, governance,
  XMPP and open questions documents.
- ADR-001 through ADR-012.
- Rust workspace with `weft-core`, `weft-db`, `weft-storage`,
  `weft-validation`, `weft-server` and `weft-cli`.
- `weft init`, `weft keygen`, `weft migrate`, `weft doctor` and `weft serve`
  commands. `weft doctor` reads cgroup v1/v2 memory and CPU limits, so a
  constrained container is checked against the 2 GiB / 2 vCPU profile. Relative config, key and storage paths are resolved against the
  configuration file directory.
- PostgreSQL migrations (`0001`–`0003`) covering nodes, keys, blobs, content,
  tags, sources, evidence, uploads, reports, moderation events, federation
  outbox/inbox, block rules, audit log, tombstones, source metadata and view
  counters.
- Content-addressed local storage with atomic writes, hash verification and
  thumbnails.
- A separate `weft-decode` worker for GIF/PNG/JPEG/WebP with dimensions, frame
  count, decoded-memory limits and JPEG thumbnails.
- Multipart upload with `open`/`review`/`closed` policies, per-client token
  buckets, quarantine, licence evidence and reports/disputes.
- Search, newest/view-based trending lists, tags, signed manifests with
  evidence, sources, byte serving with Range/ETag/Digest, and admin queue/
  approve/reject/restrict/remove/reinstate/stats/reports/storage/blocks/licence.
- Local URL/domain/tag block rules applied at upload and public retrieval.
- Source/attribution metadata, view counts, upload/decode/storage metrics,
  OpenAPI JSON, orphan cleanup and a storage quota. The search backend is behind
  a `SearchIndex` trait for a future Meilisearch adapter.
- `GET /healthz`, `GET /readyz`, `GET /metrics` and `GET /api/v1/node`.
- `flake.nix`, `shell.nix` and `.envrc` for a reproducible Nix dev shell and
  package build.
- Dockerfile, Compose profile and CI workflow. The Compose profile has not been
  run locally because Docker is unavailable here; CI has container and Compose
  jobs.
- OpenAPI 3.1 draft for the MVP REST surface.
- JSON schemas for the media manifest and node descriptor.
- `tools/check_docs.py`, `tools/graphify.sh`, `Makefile`.
- AGPL-3.0 root licence and MIT client licence.

## What does not exist

- A deployed or production-tested node.
- ActivityPub federation, replication and accounting.
- Jabu animation fix or GIF picker.

## Test inventory

| Check | Count | How to run |
|---|---|---|
| Documentation links | 40 Markdown files | `python3 tools/check_docs.py` |
| Rust tests | 32 | `cargo test --workspace` |
| PostgreSQL smoke | 1 | `WEFT_TEST_DATABASE_URL=… cargo test -p weft-server --test postgres_smoke -- --ignored` |
| Upload/review/search/media flow | 4 | `... --test upload_flow -- --ignored` |
| Real worker | 1 | `cargo test -p weft-validation --test worker_smoke` |
| Protocol golden vectors | 0 | Milestone 2 |
| Federation interop | 0 | Milestone 2 |

## Decisions in force

| ADR | Decision |
|---|---|
| ADR-001 | ActivityPub metadata layer plus the Weft Media Protocol (WMP) |
| ADR-002 | Rust, Axum, SQLx, PostgreSQL, Postgres-first search |
| ADR-003 | SHA-256 content identity, metadata separate from bytes |
| ADR-004 | Pull-based replication with signed receipts |
| ADR-005 | Anonymous uploads with a configurable moderation queue |
| ADR-006 | AGPL-3.0 server, MIT core/client |
| ADR-007 | XMPP delivery through XEP-0363 upload |
| ADR-008 | ActivityPub announces metadata, never media bytes |
| ADR-009 | Supported node profile: 2 vCPU, 2 GB, 20 GB |
| ADR-010 | Default upload policy is review |
| ADR-011 | Anonymous-upload anti-abuse: token buckets, quotas, optional PoW |
| ADR-012 | Weft naming and the Weft Media Protocol (WMP) |

## Acceptance run (2026-11-14)

The ten-step demo in [`MVP.md`](MVP.md) was run against a local PostgreSQL 16
database using the release `weft` and `weft-decode` binaries in place of the
Compose start (Docker is unavailable here). The service ran in a transient
systemd user unit capped at `MemoryMax=2G`, `MemorySwapMax=0` and
`CPUQuota=200%`, which is the supported 2 vCPU / 2 GB profile (the host itself
is larger). Disk is 38 GB free, above the 20 GB floor.

| Step | Result |
|---|---|
| Upload a two-frame GIF | `202`, hash `dbcabf76…65af7b` |
| Appears in `review`; operator approves | Queue hit, `200` |
| Search returns the active item with source | 1 hit, `https://example.org/acceptance.gif` |
| `/media/{hash}` returns exact bytes | SHA-256 matches; `ETag` and `Digest` present |
| Range request | `206`, 16 bytes |
| `/api/v1/media/{hash}` | Ed25519 signature verified with the node key |
| Remove content | Search/manifest/media return no item / `404`; row retained as `removed` |
| `/metrics` | Upload, decode, storage, media and request counters present |
| `weft doctor` under 1 GiB / 1 CPU | Refuses: reports both limits |
| `weft doctor` under 2 GiB / 2 CPU | Passes |
| `check_docs.py` and `cargo test --workspace` | Pass |

Peak cgroup memory was 4.1 MB and CPU time 0.08 s for the upload, review,
search, media, Range and removal flow with a small GIF. These numbers bound the
idle/control-plane cost only; decoding limits are covered by the worker tests.
The Compose acceptance step remains the CI gate. Its configuration was
validated locally with `docker-compose config` (Compose 5.6.0); only the image
build and health-check start are unrun.

## Next milestone

The Milestone 1 feature set works locally and the acceptance demo passes under
the supported memory/CPU caps. The remaining gate is the first CI run
(container and Compose). Federation, replication and accounting remain gated.
