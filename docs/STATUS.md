# Weft — status

Status: Milestone 0 complete; standalone MVP next. Started 2026-11-14. This is
the only status page.

## Summary

| Area | State | Evidence |
|---|---|---|
| Design documentation | Complete for Phase 1 | This docs pack and `PLAN.md` |
| Architecture and threat model | Complete for Milestone 0; review recorded | `ARCHITECTURE.md`, `THREAT_MODEL.md` |
| Design review | Approved for Milestone 0; Milestone 1 pending | `DESIGN_REVIEW.md` |
| Implementation | Milestone 0 complete; Milestone 1 not authorised | `DESIGN_REVIEW.md` §7–9 |
| Documentation link check | Passing | `python3 tools/check_docs.py` |
| Rust unit tests | 18 passing | `cargo test --workspace` |
| PostgreSQL smoke test | Passing | `cargo test -p weft-server --test postgres_smoke -- --ignored` |
| Rust workspace | Milestone 0 scaffold complete | `Cargo.toml`, `target/debug/weft` |
| Standalone node MVP | Not started | — |
| ActivityPub federation | Not started | — |
| Media replication | Not started | — |
| Contribution accounting | Not started | — |
| Licence/moderation workflow | Not started | — |
| Jabu GIF picker | Not started | — |
| Jabu animated-attachment fix | Not started | Tracked in `PLAN.md` |

The Milestone 0 server scaffold, schema, container definition and local health
endpoints exist. There is **no upload pipeline, search, media serving,
federation, replication or deployed node** yet. Do not claim otherwise in README,
issues or release notes.

## What exists

- `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSING.md`.
- Architecture, protocol, federation, API, data-model, config, MVP, threat
  model, design review, privacy, operations, content-licensing, governance,
  XMPP and open questions documents.
- ADR-001 through ADR-012.
- Rust workspace with `weft-core`, `weft-db`, `weft-storage`,
  `weft-validation`, `weft-server` and `weft-cli`.
- `weft init`, `weft keygen`, `weft migrate` and `weft serve` commands.
- First PostgreSQL migration (`0001_init.sql`) covering nodes, keys, blobs,
  content, tags, sources, evidence, uploads, reports, moderation events,
  federation outbox/inbox, block rules and audit log.
- Content-addressed local storage with atomic writes and hash verification.
- `GET /healthz`, `GET /readyz`, `GET /metrics` and `GET /api/v1/node`.
- Dockerfile, Compose profile and CI workflow (the Compose profile has not
  been run locally because Docker is unavailable in this environment; CI has a
  compose job).
- OpenAPI 3.1 draft for the MVP REST surface.
- JSON schemas for the media manifest and node descriptor.
- `tools/check_docs.py`, `tools/graphify.sh`, `Makefile`.
- AGPL-3.0 root licence and MIT client licence.

## What does not exist

- Upload, quarantine, search, manifests or media serving.
- A deployed or production-tested node.
- Jabu animation fix or GIF picker.

## Test inventory

| Check | Count | How to run |
|---|---|---|
| Documentation links | 40 Markdown files | `python3 tools/check_docs.py` |
| Rust unit tests | 18 | `cargo test --workspace` |
| PostgreSQL smoke | 1 | `WEFT_TEST_DATABASE_URL=… cargo test -p weft-server --test postgres_smoke -- --ignored` |
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

## Next milestone

Milestone 0 is complete. The next implementation step is the standalone MVP in
[`MVP.md`](MVP.md), but Milestone 1 still requires the maintainer to approve the
design review for that scope. See [`DESIGN_REVIEW.md`](DESIGN_REVIEW.md) and
[`PLAN.md`](PLAN.md).
