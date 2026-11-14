# Weft — federation over ActivityPub

Status: design for Milestone 2. The server must remain fully usable with
federation disabled.

Weft uses ActivityPub for **metadata and events**, not for media bytes. Every
node is an independent ActivityPub actor with its own keys and policies. There is
no central Weft service and no mandatory relay.

## 1. Actor

Each node exposes a `Service` actor:

```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://weft.network/ns"
  ],
  "id": "https://gifs.example/actor",
  "type": "Service",
  "preferredUsername": "node",
  "name": "Example GIF node",
  "summary": "A small community node",
  "url": "https://gifs.example",
  "inbox": "https://gifs.example/inbox",
  "outbox": "https://gifs.example/outbox",
  "followers": "https://gifs.example/followers",
  "following": "https://gifs.example/following",
  "publicKey": {
    "id": "https://gifs.example/actor#key-1",
    "owner": "https://gifs.example/actor",
    "publicKeyPem": "-----BEGIN PUBLIC KEY-----..."
  },
  "weft:node": {
    "descriptor": "https://gifs.example/.well-known/weft",
    "software": "weft-server/0.1.0",
    "capabilities": ["metadata", "replication"],
    "mediaProtocol": "weft-mdp/0.1"
  }
}
```

- The actor URL is the stable node identity.
- The node descriptor carries the machine-readable limits and capabilities.
- Keys are Ed25519. `publicKeyPem` is used for HTTP Signatures; the node
  descriptor also exposes the raw base64url key for Weft-native verification.
- Key rotation adds a new key and marks the old key `validUntil`. A revoked key
  stays listed with `revokedAt` so old signatures can be classified as
  historical, not current.
- The JSON-LD namespace is provisional. See
  [`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) Q2.

## 2. Discovery

Several mechanisms are complementary. None of them grants trust.

1. **Bootstrap list.** A configured list of node origins or actor URLs. It must
   not be the only way to join the network.
2. **Peer exchange.** `GET /api/v1/nodes` returns known descriptors; a node may
   share its peer list subject to policy.
3. **ActivityPub references.** Actors and activities link to peer actors and
   manifests.
4. **Manual configuration.** An operator can add a node by URL.

Discovery rules:

- Fetch `/.well-known/weft` and the actor; verify signatures before using
  metadata.
- Cache descriptors for a bounded time and refresh on failure.
- Do not follow arbitrary redirects; restrict to HTTPS and reject private
  addresses.
- A discovered node is not automatically indexed, followed or replicated.
- Bootstrap nodes have no special authority; the network must survive their
  disappearance.

## 3. Content objects

A published content record is an ActivityPub `Document`. It points to the
manifest and bytes; it never contains the bytes.

```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://weft.network/ns"
  ],
  "id": "https://gifs.example/o/3a7bd3e2360a...",
  "type": "Document",
  "attributedTo": "https://gifs.example/actor",
  "published": "2026-11-14T00:00:00Z",
  "updated": "2026-11-14T00:00:00Z",
  "name": "Surprised cat",
  "summary": "Reaction GIF",
  "mediaType": "image/gif",
  "url": "https://gifs.example/media/3a7bd3e2360a...",
  "weft:contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "weft:manifest": "https://gifs.example/media/3a7bd3e2360a.../manifest",
  "weft:size": 1234567,
  "weft:width": 480,
  "weft:height": 270,
  "weft:frames": 42,
  "weft:tags": ["cat", "reaction"],
  "weft:licence": {
    "claim": "CC0-1.0",
    "spdx": "CC0-1.0",
    "status": "unverified",
    "evidence": ["https://example.org/original"]
  }
}
```

- The ActivityPub `id` is a canonical HTTPS URL.
- `weft:contentId` is the immutable hash identity.
- `digest` may include `sha-256=<base64>` for standard consumers.
- `url` points to bytes; `weft:manifest` points to the signed metadata.
- Metadata may be updated; bytes may not.

## 4. Activities

### 4.1 Create, Update, Delete

- `Create` publishes a `Document` to the outbox and delivers it to followers and
  explicitly addressed inboxes.
- `Update` replaces metadata (title, tags, licence claim, moderation state). The
  bytes and `weft:contentId` must not change.
- `Delete` removes the public object and creates a tombstone. It must not include
  the media bytes.
- A node may send `Update` to correct an error, add evidence or apply a
  moderation decision.

### 4.2 Reports and flags

Use the standard `Flag` activity for user reports:

```json
{
  "id": "https://gifs.example/reports/01J...",
  "type": "Flag",
  "actor": "https://gifs.example/actor",
  "object": "https://peer.example/o/3a7bd3e2360a...",
  "content": "Licence evidence does not match the claimed source.",
  "weft:reason": "licence",
  "published": "2026-11-14T00:00:00Z"
}
```

`weft:reason` values: `licence`, `illegal`, `spam`, `malware`, `privacy`,
`other`. A `Flag` is an allegation, not a decision.

### 4.3 Moderation events

Define an application-level event object. It is published by the node that made
the decision; other nodes apply their own policy.

```json
{
  "@context": [
    "https://www.w3.org/ns/activitystreams",
    "https://weft.network/ns"
  ],
  "id": "https://gifs.example/events/01J...",
  "type": "weft:ModerationEvent",
  "weft:category": "CONTENT_RESTRICTED",
  "weft:contentId": "urn:weft:sha256:3a7bd3e2360a...",
  "weft:object": "https://gifs.example/o/3a7bd3e2360a...",
  "weft:issuer": "https://gifs.example/actor",
  "weft:issuedAt": "2026-11-14T00:00:00Z",
  "weft:scope": "local",
  "weft:reason": "licence_dispute",
  "weft:evidence": ["https://gifs.example/disputes/01J..."],
  "weft:reference": "https://example.org/claim"
}
```

Categories: `CONTENT_REPORTED`, `LICENCE_DISPUTED`, `CONTENT_RESTRICTED`,
`CONTENT_BLOCKED`, `CONTENT_REMOVED`, `CONTENT_REINSTATED`, `NODE_SUSPENDED`.
A node may add categories only through a versioned extension.

Rules:

- Each event is signed by the issuing node.
- A node must not automatically apply another node's event. It may use it as
  input to its own policy.
- `scope` describes the issuer's intent, not a global obligation.
- Do not include personal data, raw reports or illegal-content samples.

### 4.4 Node blocking

- `Block` may be delivered to a peer to state that the sender will no longer
  federate with it.
- A node maintains local allow/block lists.
- Community blocklists are opt-in and never mandatory.
- Blocking is not a global ban; other nodes decide independently.

## 5. Inbox processing

For each inbound activity:

1. Enforce method, content type, body size and rate limits.
2. Fetch the actor and key if not cached; verify HTTPS and host policy.
3. Verify the HTTP Signature and `Digest`.
4. Reject timestamps outside the allowed clock skew and reused nonces.
5. Deduplicate by activity `id` in a persistent replay cache.
6. Validate the JSON-LD shape and the `weft` extension fields.
7. Apply local policy: is the actor blocked? Is the content type accepted? Is
   the licence claim acceptable? Is the media size within limits?
8. Store the event in the inbox/moderation log.
9. Index metadata only if policy allows; never download bytes automatically from
   an inbox activity.
10. Return `202` even when the activity is accepted but ignored, to avoid
    retry storms. Return `4xx` for malformed or unauthenticated input.

Media download is always a separate, scheduled, policy-controlled action.

## 6. Signatures

Weft uses Ed25519 keys for its own protocol. ActivityPub HTTP Signatures are used
for inbound and outbound requests:

- `Signature` header with `keyId`, `algorithm`, `headers`, `signature`;
- `Digest` for requests with a body;
- `Host`, `Date` and `(request-target)` in the signed header set;
- five-minute clock skew;
- nonce cache for Weft extensions.

Compatibility with implementations that only accept RSA is unresolved. See
[`OPEN_QUESTIONS.md`](OPEN_QUESTIONS.md) Q3. The first federation milestone
targets Weft-aware nodes only and must say so in `STATUS.md`.

## 7. Outbox and followers

- The outbox is an ordered collection of public `Create`/`Update`/`Delete`
  activities.
- `followers` is a collection of actors that asked for announcements. A node may
  accept, ignore or rate-limit follow requests.
- Following is a delivery preference, not a trust relationship.
- A node must be able to publish without followers and without a relay.

## 8. Federation policy

Per-node configuration:

```toml
[federation]
enabled = true
bootstrap_nodes = ["https://gifs.example", "https://another.example"]
require_http_signatures = true
allow = []            # empty means "discover, but apply local policy"
block = ["https://spam.example"]
max_inbox_bytes = 262144
deliver_to_followers = true
```

At minimum a node can:

- accept metadata from allowed peers;
- ignore metadata it does not want;
- refuse replication requests;
- block a peer;
- defederate without affecting other peers.

## 9. Privacy

- Never federate contributor JIDs, emails, raw IPs or private account data.
- Do not include uploader IPs or user agents in activities.
- Use node-level actors; do not create an ActivityPub actor per anonymous
  contributor by default.
- Optional pseudonymous contributor keys must be separate from node keys and
  must not be required.
- Federation logs must be redacted and retained for the shortest practical time.

## 10. Interoperability notes

- ActivityPub is used for discovery and event delivery; media retrieval is
  Weft-specific.
- Generic fediverse servers will not understand `weft:*` fields and are not
  expected to host media.
- A node should expose standard `Document` fields (`name`, `summary`,
  `mediaType`, `url`, `digest`) so other tools can at least read metadata.
- Do not publish misleading `Mastodon-compatible` claims.

## 11. Open points

- Final JSON-LD namespace and context hosting.
- Exact HTTP Signature algorithm profile and RSA compatibility.
- Whether `Flag` alone is sufficient for licence disputes or a dedicated object
  is needed.
- Relay or gossip discovery beyond peer exchange.
- Inbox retention and moderation-event retention defaults.
