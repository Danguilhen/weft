# ADR-014 — JSON-LD namespace and context hosting

- Status: Accepted
- Date: 2026-11-14
- Affects: federation, protocol, documentation, operations

## Context

Weft extends ActivityPub with `weft:` terms. JSON-LD requires a stable context
URL so remote nodes interpret those terms consistently. The project does not yet
own a public domain, and a namespace must not point at a domain that may change
or disappear. Changing the namespace after federation would change the meaning
of existing objects and break cached contexts.

Q2 asked which JSON-LD namespace to use. The project chose a w3id identifier so
the namespace is independent of a future brand domain.

## Decision

- Canonical term namespace: `https://w3id.org/weft/ns#`.
- Context document URL used in `@context`: `https://w3id.org/weft/ns`.
- The context maps the `weft` prefix to the namespace. Terms are used as compact
  IRIs such as `weft:contentId`, `weft:manifest` and `weft:ModerationEvent`.
- A canonical copy lives at [`../context/weft.jsonld`](../context/weft.jsonld).
  Nodes pin this local copy instead of fetching remote contexts.
- The w3id redirect is registered through `perma-id/w3id.org` before federation
  is enabled. It must point at a stable, versioned HTTPS document served as
  `application/ld+json` (or `application/json`), allow CORS, and cache for a long
  time.
- If the project later acquires a domain, it may mirror the context there, but
  the canonical namespace remains the w3id URL. `weft.network` is not used until
  the project controls it.
- Namespace evolution is additive. Terms are never renamed or removed; a
  breaking change requires a new namespace and a new ADR.
- Nodes accept only the pinned Weft context, the standard ActivityStreams
  context and explicitly allowed extensions. Unknown remote contexts are
  rejected or ignored, never fetched.

## Consequences

- Positive: a stable identifier that does not depend on owning a domain.
- Positive: one canonical context for all nodes and clients.
- Negative: the project depends on the w3id service for redirects.
- Negative: the context must be published before federation can be enabled.
- Follow-up: submit the w3id redirect, host the context, and add a startup check
  that the pinned context matches the namespace.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| `https://weft.network/ns#` now | The project does not own the domain; a placeholder would be unresolvable |
| GitHub raw content URL | Not a stable protocol namespace; content type and ownership can change |
| A URN namespace | JSON-LD contexts are expected to be dereferenceable HTTPS URLs |
| No extension namespace | Weft moderation and manifest terms need stable IRIs |

## References

- [`../OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) Q2
- [`../FEDERATION.md`](../FEDERATION.md)
- [`../context/weft.jsonld`](../context/weft.jsonld)
