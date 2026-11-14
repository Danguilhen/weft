# Weft

Weft is a community-owned, federated network for GIFs and animated media. It is
built as an independent service with its own API and federation protocol, and is
designed to be embedded in clients such as Jabu without depending on a central
commercial provider.

The working name **Weft** replaces the earlier **OpenGIF** codename. The public
name and domain are not final yet; see
[`docs/OPEN_QUESTIONS.md`](docs/OPEN_QUESTIONS.md).

## Status

**Design phase.** The repository currently contains the architecture, protocol,
API, security and planning documents plus agent tooling. No server or client code
has been written yet.

- What exists and what does not: [`docs/STATUS.md`](docs/STATUS.md)
- Open work and phase order: [`docs/PLAN.md`](docs/PLAN.md)
- Documentation index: [`docs/README.md`](docs/README.md)

## Why

Google shut down the Tenor API on 30 June 2026. The remaining GIF APIs are
commercial, key-gated and impose attribution and policy terms. A community-owned
network should not depend on any one of them.

Weft separates two concerns:

1. **Metadata and federation** over ActivityPub: node discovery, announcements,
   tags, licence claims, moderation events.
2. **Media distribution** over a dedicated HTTPS protocol: bytes, manifests,
   replication, integrity verification, bandwidth accounting and caching.

Large media files never travel through ActivityPub. ActivityPub announces and
describes content; the media protocol delivers it.

## Principles

- Open federation and independent administration.
- No central authority, no registration authority and no mandatory approval.
- No blockchain, cryptocurrency, tokens or mandatory payments.
- Anonymous browsing and anonymous contributions by default.
- Minimal collection of personal data; contributor identities never federate.
- Content integrity through SHA-256 content addressing.
- Per-node moderation, licence acceptance and replication policy.
- Cooperative, demand-driven replication that works on small self-hosted nodes.
- Documented, versioned APIs and protocols that other clients can implement.

## Planned architecture

```text
                    ActivityPub (metadata)
   ┌──────────┐   Create / Update / Delete   ┌──────────┐
   │  Node A  │◀────────────────────────────▶│  Node B  │
   │          │   Flag / ModerationEvent     │          │
   └────┬─────┘                              └─────┬────┘
        │                                          │
        │        Weft Media Distribution (bytes)   │
        │   manifests, pull replication, receipts   │
        └──────────────────────────────────────────┘
```

- ActivityPub actor per node, signed with Ed25519 node keys.
- Content bytes identified by `urn:weft:sha256:<hash>`.
- Media served from `/media/{hash}` with a signed manifest.
- Replication is pull-based and voluntary.
- Resource accounting uses locally calculated scores and signed receipts, never
  a global reputation ledger.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full design.

## Planned repository layout

```text
crates/
  weft-core/          Domain types, hashing, licence states, policy
  weft-db/            PostgreSQL schema and queries
  weft-storage/       Local filesystem, optional S3/MinIO
  weft-validation/    Decode limits, thumbnails, metadata stripping
  weft-federation/    ActivityPub, HTTP signatures, inbox/outbox
  weft-server/        Axum REST API, admin, metrics
  weft-client/        MIT-licensed Rust client for Jabu and other apps
  weft-cli/           init, keygen, migrate, import, verify, reindex
deploy/               Dockerfile, compose, reverse-proxy examples
docs/                 Design, protocol, API, operations, ADRs
tools/                Documentation checks, graphify wrapper
```

## Documentation

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — components and data flow
- [`docs/PROTOCOL.md`](docs/PROTOCOL.md) — media distribution protocol
- [`docs/FEDERATION.md`](docs/FEDERATION.md) — ActivityPub integration
- [`docs/API.md`](docs/API.md) — REST API
- [`docs/MVP.md`](docs/MVP.md) — first standalone milestone
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — security and abuse analysis
- [`docs/OPERATIONS.md`](docs/OPERATIONS.md) — self-hosting and maintenance
- [`docs/XMPP.md`](docs/XMPP.md) — Jabu integration plan
- [`docs/adr/README.md`](docs/adr/README.md) — decision records

## Licensing

- Server, federation and CLI crates: **AGPL-3.0-or-later**.
- `weft-core` and `weft-client`: **MIT**, so clients can embed them freely.

See [`LICENSING.md`](LICENSING.md). Content licensing is a separate concern:
see [`docs/CONTENT_LICENSING.md`](docs/CONTENT_LICENSING.md).

## Contributing

Read [`AGENTS.md`](AGENTS.md) first. It defines the documentation rules, the
phase order, the non-negotiables and the verification commands. The design is
deliberately documented before implementation so that contributors and agents
can work from the same specification.

Security reports: see [`SECURITY.md`](SECURITY.md).
