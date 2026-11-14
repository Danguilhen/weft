# ADR-013 — ActivityPub signature profile

- Status: Accepted
- Date: 2026-11-14
- Affects: federation, configuration, operations, threat model

## Context

ActivityPub implementations in the wider fediverse generally use the
draft-cavage HTTP Signatures scheme with RSA-2048 keys and RSA-SHA256. Ed25519
verification exists in some newer stacks, but it is not a safe baseline: an
Ed25519-only node would fail to interoperate with many Mastodon-class peers.
Weft already uses Ed25519 for WMP manifests, where it controls both ends of the
protocol.

Q3 asked whether to use Ed25519 only or add an RSA compatibility key. The project
does not want a walled garden, so the compatibility option is required.

## Decision

- ActivityPub actors use an **RSA-2048** key with key id
  `<actor>#ap-key-1`, published as `publicKeyPem` in the actor.
- Outbound ActivityPub requests are signed with HTTP Signatures using
  `rsa-sha256`. Inbound signatures are verified with the peer's advertised RSA
  key.
- The signed header set includes at least `(request-target)`, `host`, `date` and
  `digest` for requests with a body.
- The existing Ed25519 key remains the **WMP manifest key** at
  `<actor>#key-1`; it is exposed in the node descriptor. It is not required for
  ActivityPub interoperability.
- Weft-native nodes may also verify Ed25519 WMP manifests and may advertise an
  Ed25519 key for future use, but ActivityPub delivery must not depend on it.
- Key rotation and revocation apply independently to both keys. New keys are
  added before old keys expire; old keys carry `validUntil` and `revokedAt` so a
  signature can be classified as historical rather than current.
- Do not claim RFC 9421 support until the fediverse ecosystem adopts it. The
  compatibility target is draft-cavage HTTP Signatures.

## Consequences

- Positive: interoperability with the existing fediverse instead of a
  Weft-only network.
- Positive: WMP keeps its compact Ed25519 signatures and existing key id.
- Negative: two key lifecycles and a larger RSA dependency.
- Negative: RSA signing is slower than Ed25519; this is negligible at node
  scale.
- Follow-up: add RSA key generation to `weft init`, document separate key files,
  and add golden vectors for both RSA HTTP Signatures and Ed25519 manifests.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Ed25519 only | Walled garden; many ActivityPub peers reject or ignore it |
| RSA only for every signature | Loses the existing compact WMP key and forces a protocol change |
| RFC 9421 with Ed25519 | Cleaner modern design, but poor current fediverse interoperability |
| No HTTP Signatures | Unauthenticated federation |

## References

- [`../OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) Q3
- [`../FEDERATION.md`](../FEDERATION.md)
- [`../THREAT_MODEL.md`](../THREAT_MODEL.md)
- [`../PLAN.md`](../PLAN.md) Milestone 2
