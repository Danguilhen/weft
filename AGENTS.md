# AGENTS.md

# Weft — agent and contributor guide

This document tells agents and contributors how the repository works today. It
is not a status page and not a roadmap.

- **`docs/STATUS.md` is the single source of truth** for what exists and what is
  verified.
- **`docs/PLAN.md` is the open-work queue.** It contains open work only. Delete a
  bullet when it is done instead of appending progress notes.
- **`docs/adr/` records decisions.** Change an ADR in place when reality changes;
  write a new ADR for a new decision.
- **`docs/OPEN_QUESTIONS.md` records unresolved choices.** Do not silently
  resolve a listed question in code.

**Starting work:** read `docs/STATUS.md`, then `docs/PLAN.md`, then the ADR that
owns the area you are touching. If the task conflicts with an ADR, stop and ask.

## Project overview

Weft is a standalone, federated, community-owned network for GIFs and animated
media. It is designed for the XMPP ecosystem first — Jabu is the first client —
but its REST API and federation protocol are independent of any one client.

The former working name was **OpenGIF**. Use **Weft** everywhere in the
repository. `OpenGIF` may appear only in historical context.

The project is in the **design phase**. The deliverable for the current phase is
the documentation pack; implementation starts with the standalone MVP in
`docs/MVP.md`.

**Implementation gate:** no code is authorised until `docs/DESIGN_REVIEW.md`
records an approved verdict. Read the review findings and conditions before
scaffolding anything. The current verdict is conditional approval with sign-off
pending.

## Non-negotiables

1. **No blockchain, cryptocurrency, tokens, transferable credit or mandatory
   payments.** Cooperative accounting is a local resource-allocation signal, not
   money.
2. **No central authority.** No registration authority, no mandatory global
   allow-list, no global reputation database, no central licence authority.
3. **Independent moderation.** Every node decides what it accepts, indexes,
   replicates and serves. A blocklist is local or opt-in, never mandatory.
4. **Privacy by default.** Do not propagate XMPP JIDs, names, emails, raw IP
   addresses or contributor identities through federation. Do not add analytics,
   third-party embeds or tracking parameters.
5. **Content integrity.** Bytes are identified by SHA-256. Metadata never
   substitutes for the hash.
6. **Two layers.** ActivityPub carries metadata and events; the media protocol
   carries bytes. Never put large media through ActivityPub.
7. **Standalone first.** The server must work without federation, without a
   commercial GIF provider and without a central Weft service.
8. **Small-node first.** The MVP must run on a modest VPS or home server.
9. **Licensing split.** Server/federation/CLI code is AGPL-3.0-or-later;
   `weft-core` and `weft-client` are MIT. Keep the client crates free of
   server-only dependencies.
10. **Document assumptions.** If a protocol field, default or legal assumption is
    not documented, it is not decided.

## Current phase

**Phase 0 — design and repository foundation.**

- The docs under `docs/` are the specification for Phase 1.
- No Rust workspace exists yet. Do not add production code before the Phase 1
  tasks in `docs/PLAN.md` are unblocked.
- The first implementation milestone is the standalone MVP in `docs/MVP.md`.

## Planned repository layout

```text
crates/
  weft-core/          Domain types, hashing, licence states, policy. MIT.
  weft-db/            PostgreSQL schema, migrations and queries. AGPL.
  weft-storage/       Local filesystem and optional S3/MinIO. AGPL.
  weft-validation/    Decode limits, thumbnails, metadata stripping. AGPL.
  weft-federation/    ActivityPub, HTTP signatures, inbox/outbox. AGPL.
  weft-server/        Axum REST API, admin surface, metrics. AGPL.
  weft-client/        Typed Rust client for Jabu and other apps. MIT.
  weft-cli/           init, keygen, migrate, import, verify, reindex. AGPL.
deploy/               Dockerfile, compose, reverse proxy examples.
docs/                 Design, protocol, API, operations, ADRs.
tools/                Documentation checks, graphify wrapper.
```

`weft-core` must not depend on `weft-server`, `weft-db` or any HTTP framework.
`weft-client` must not depend on the server side.

## Development commands

There is no Rust workspace yet. The current checks are:

```bash
python3 tools/check_docs.py      # relative links and required docs
./tools/graphify.sh              # optional knowledge graph refresh
```

See [`tools/README.md`](tools/README.md).

Once the Phase 1 workspace exists, the expected commands are:

```bash
nix develop                       # or the documented Rust toolchain
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
docker compose -f deploy/docker-compose.yml up -d
```

Do not add a toolchain, formatter or linter that is not documented here.

## Documentation rules

1. `docs/STATUS.md` is the only status page. Update it when behaviour changes.
2. `docs/PLAN.md` is a backlog: open work only. Delete finished bullets.
3. `docs/adr/` owns decisions. Do not rewrite an accepted decision without a new
   ADR that supersedes it.
4. `docs/OPEN_QUESTIONS.md` owns unresolved choices. Link the affected question
   from the task instead of deciding it in a pull request.
5. `README.md` stays short: overview, status link, architecture summary, build
   and test, licensing.
6. Do not duplicate tables, endpoint lists or protocol details across documents.
   Link the owning document.
7. Keep examples executable or clearly marked illustrative.
8. Update `docs/openapi.yaml` when the REST surface changes.
9. Update `docs/schemas/` when a wire schema changes.
10. Never describe a feature as working unless `docs/STATUS.md` records evidence.

## Implementation rules

- Read the owning ADR before changing a protocol, storage or moderation choice.
- Keep wire formats versioned. A breaking change needs a new version or an ADR.
- Use established libraries: do not invent cryptographic primitives, canonical
  JSON or signature encodings.
- Canonical JSON is RFC 8785 (JCS). Signatures are Ed25519 over the encoded
  bytes. Use base64url without padding for transport.
- Validate uploads by decoding, never by trusting extensions or client MIME
  types. Enforce file size, pixel count, frame count, decoded memory and time.
- Do not store raw IP addresses. If rate limiting needs an address, use a salted,
  rotating hash with a documented retention period.
- Logs must not contain media bytes, upload bodies, tokens or contributor
  identifiers. Redact URLs that contain credentials.
- Every public endpoint needs a documented auth model, rate limit and error
  shape.
- Tests must not silently skip when Postgres or another required service is
  missing. Follow the repository's test convention once it exists.

## Testing and verification

The design phase is verified by the docs check:

```bash
python3 tools/check_docs.py
```

When code exists, the minimum gate for a change is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Protocol changes additionally require one of:

- a golden-vector test for signatures, manifests or receipts;
- an integration test with two in-process nodes;
- a documented manual interoperability test in `docs/STATUS.md`.

## Definition of done

A task is done only when:

1. The behaviour is implemented or the document is complete.
2. The relevant tests or checks pass.
3. `docs/STATUS.md` reflects the new reality (for code).
4. The open bullet was removed from `docs/PLAN.md`.
5. Any decision made along the way is recorded in an ADR or an existing ADR was
   updated.
6. Any new open question is added to `docs/OPEN_QUESTIONS.md`.
7. Links still pass `python3 tools/check_docs.py`.

## Graphify (optional)

Weft can be indexed into a code knowledge graph to reduce exploration tokens once
the Rust workspace exists.

```bash
npm install -g graphifyy --prefix ~/.local
export PATH="$HOME/.local/bin:$PATH"

./tools/graphify.sh
graphify query graphify-out/.graphify/graph.json "where is the upload validation pipeline"
```

The wrapper excludes build output, generated state, docs and vendored code. It
writes to `graphify-out/`, which is gitignored, and is safe to rerun after
structural changes. If `graphify` is not installed, the script explains how to
install it and exits non-zero.

## Commit and release policy

- Keep commits small and scoped to one task or document.
- Do not push tags, publish crates or publish container images without explicit
  maintainer approval.
- Add a changelog entry when a release process exists.
- Do not rewrite published history.

## Security and privacy

Read `docs/THREAT_MODEL.md` and `docs/PRIVACY.md` before touching upload,
federation or moderation code. Security-sensitive changes need an explicit
review note in the pull request. Do not open public issues for vulnerabilities;
follow `SECURITY.md`.
