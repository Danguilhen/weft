# Weft — governance

Status: initial project governance. The network's governance is deliberately
different from the software project's governance.

## 1. Two kinds of governance

- **The network** has no central authority. Node operators decide what their
  nodes accept, index, replicate and serve. No organisation can force a node to
  join, leave or accept content.
- **The software project** needs maintainers to merge changes, publish releases
  and protect the name. That role is stewardship, not control of the network.

Nothing in this document creates a central moderation authority for content.
Moderation events are statements by individual nodes.

## 2. Principles

- Open participation and open source.
- No blockchain, cryptocurrency, tokens or mandatory payments.
- No mandatory account, registration authority or global allow-list.
- Privacy and data minimisation by default.
- Independent moderation and licence acceptance.
- Small self-hosted nodes are first-class.
- Decisions are documented, reversible where possible, and open to challenge.
- Forking is allowed and expected; the licence must not be used to trap users.

## 3. Maintainers

Maintainers are contributors who have been given merge or release rights. The
initial set is the project founder plus anyone explicitly added by consensus.

Responsibilities:

- review changes against `AGENTS.md` and the ADRs;
- keep `STATUS.md`, `PLAN.md` and the design documents honest;
- publish releases and security advisories;
- protect the project name and domain;
- handle security reports and project-level conduct issues.

Maintainers are not moderators of independently operated nodes.

## 4. Decisions

- **Small changes:** lazy consensus in the pull request.
- **Protocol, storage, security or governance changes:** an ADR is required
  before merge.
- **Irreversible or public-commitment changes** (name, domain, licence, release
  policy): explicit maintainer consensus and a public note.
- **Disagreement:** discuss in the open; if consensus fails, the maintainers
  decide and record the rationale. A minority may fork under the licence.

ADRs are numbered, immutable once accepted, and superseded rather than rewritten
when a decision changes. The template is in `docs/adr/TEMPLATE.md`.

## 5. Releases

- Releases are cut from `main` after the release gate in `PLAN.md`.
- A release notes file lists protocol versions, schema migrations and breaking
  changes.
- Binaries and container images should be signed once the release process exists.
- Do not publish a release that claims federation interoperability without an
  interop test recorded in `STATUS.md`.

## 6. Funding

The project may accept donations and grants for infrastructure and maintenance.
It must not:

- sell priority access to the network;
- create transferable credits or tokens;
- require payment to run a node or browse;
- let a funder control protocol decisions.

A funded maintainer discloses the funding in the repository.

## 7. Name, domain and marks

The public name **Weft** is provisional until a name and domain check is
complete. The project should own a domain and a code hosting organisation, not a
central service. The domain must not become a mandatory part of the protocol.

Do not register a domain, crate name or trademark on behalf of the project
without maintainer agreement.

## 8. Security

Security reports follow [`../SECURITY.md`](../SECURITY.md). Maintainers may make
a private fix and disclose after a coordinated window. A security fix may bypass
the normal review quorum when delay would harm users, but it must be reviewed
afterwards.

## 9. Project conduct

Be civil and technical. Harassment, discrimination and disclosure of private
information are not acceptable. Maintainers may moderate project spaces (issues,
chat, pull requests). This does not give them power over independent nodes.

## 10. Conflict of interest

Maintainers disclose employment, funding or commercial interests that could
influence a protocol decision. A maintainer with a direct conflict should recuse
from the final decision.

## 11. Forking and continuity

The AGPL server and MIT client licences allow forks. Forks should use a different
name and must not imply endorsement. The protocol specification is intended to
outlive any one implementation; a fork that follows the spec should be able to
federate.

## 12. Amendment

Amend this document through an ADR and a public review period. Keep the history
in the ADR.
