# Weft — status

Status: design phase, started 2026-11-14. This is the only status page.

## Summary

| Area | State | Evidence |
|---|---|---|
| Design documentation | Complete for Phase 1 | This docs pack and `PLAN.md` |
| Documentation link check | Passing | `python3 tools/check_docs.py` |
| Rust workspace | Not started | — |
| Standalone node MVP | Not started | — |
| ActivityPub federation | Not started | — |
| Media replication | Not started | — |
| Contribution accounting | Not started | — |
| Licence/moderation workflow | Not started | — |
| Jabu GIF picker | Not started | — |
| Jabu animated-attachment fix | Not started | Tracked in `PLAN.md` |

There is **no server code, database schema, container image or deployed node**
yet. Do not claim otherwise in README, issues or release notes.

## What exists

- `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `SECURITY.md`, `LICENSING.md`.
- Architecture, protocol, federation, API, data-model, config, MVP, threat
  model, privacy, operations, content-licensing, governance, XMPP and open
  questions documents.
- ADR-001 through ADR-008.
- OpenAPI 3.1 draft for the MVP REST surface.
- JSON schemas for the media manifest and node descriptor.
- `tools/check_docs.py`, `tools/graphify.sh`, `Makefile`, docs CI workflow.
- AGPL-3.0 root licence and MIT client licence.

## What does not exist

- `Cargo.toml`, any crate, migrations or executable.
- PostgreSQL schema or storage backend.
- Upload/validation pipeline.
- ActivityPub actor, inbox, signatures or discovery.
- Replication, receipts or accounting.
- Moderation or licence-dispute workflow.
- Docker Compose stack or operator tooling.
- Jabu animation fix or GIF picker.

## Test inventory

| Check | Count | How to run |
|---|---|---|
| Documentation links | — | `python3 tools/check_docs.py` |
| Rust unit/integration | 0 | — |
| Protocol golden vectors | 0 | — |
| Federation interop | 0 | — |

## Decisions in force

| ADR | Decision |
|---|---|
| ADR-001 | ActivityPub metadata layer plus a separate media distribution protocol |
| ADR-002 | Rust, Axum, SQLx, PostgreSQL, Postgres-first search |
| ADR-003 | SHA-256 content identity, metadata separate from bytes |
| ADR-004 | Pull-based replication with signed receipts |
| ADR-005 | Anonymous uploads with a configurable moderation queue |
| ADR-006 | AGPL-3.0 server, MIT core/client |
| ADR-007 | XMPP delivery through XEP-0363 upload |
| ADR-008 | ActivityPub announces metadata, never media bytes |

## Next milestone

Milestone 0 in [`PLAN.md`](PLAN.md): resolve the blocking questions, scaffold the
Rust workspace, add CI and the Docker Compose skeleton. Then start the standalone
MVP in [`MVP.md`](MVP.md).
