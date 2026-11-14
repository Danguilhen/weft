# Weft — content licensing

Status: design. This document is about the licence of uploaded media, not the
software licence. Software licensing is in [`../LICENSING.md`](../LICENSING.md).

## 1. The problem

A cryptographic hash proves that bytes are the same bytes. It does **not** prove
who created them or under what licence they were published. An uploader can
falsely label copyrighted content as public domain or Creative Commons.

Weft must therefore separate:

- what the uploader **claims**;
- what **evidence** exists;
- what this node has **verified**;
- whether a claim is **disputed**;
- what the node's **policy** accepts.

No node may present an unverified claim as legally certified.

## 2. Fields

| Field | Meaning |
|---|---|
| `claim` | Human-readable statement, e.g. "CC0 1.0" |
| `spdx` | SPDX identifier when it exists |
| `url` | Licence URL, preferably the canonical deed |
| `status` | Verification state (see §3) |
| `evidence` | URLs and notes supporting the claim |
| `verifiedBy` | Node that performed a verification, if any |
| `verifiedAt` | Time of verification |
| `dispute` | Optional reference to a dispute event |

Use SPDX identifiers where possible: `CC0-1.0`, `CC-BY-4.0`,
`CC-BY-SA-4.0`, `MIT`, `Apache-2.0`, and so on. For custom terms, use `NOASSERTION`
and a URL.

## 3. Statuses

```text
unverified           The uploader made a claim; nothing was checked.
evidence_provided    Evidence URLs are present but not reviewed.
verified_evidence    This node reviewed the evidence; it is plausible.
disputed             A credible dispute is open; distribution may be suspended.
invalidated          The claim was rejected or a takedown applies.
```

Rules:

- `unverified` is the default.
- `verified_evidence` means "this node checked the evidence", not "a court
  confirmed the copyright". UI wording must say so.
- `disputed` may trigger local restriction; the node decides.
- `invalidated` must stop new replication and public serving where required.
- A node may accept content only from a configured set of licences.
- A node may accept `unverified` content but must display the status.

## 4. Evidence

Acceptable evidence includes:

- the original creator's website or post;
- the original publication URL with a matching licence statement;
- a licence declaration attached to the source;
- written permission (stored privately, not published);
- public-domain documentation;
- an authoritative release or archive.

Evidence handling:

- Store the URL and, when possible, a hash of the fetched snapshot.
- Do not republish private correspondence or personal data.
- Do not fetch evidence URLs with user credentials or from private networks.
- A dead link is not proof of fraud; it weakens the evidence.
- Evidence does not override a valid takedown or legal order.

## 5. Disputes

Report flow:

1. A person submits `/api/v1/licence-dispute` with the content ID, a claim, and
   optional evidence.
2. The node records a local `LICENCE_DISPUTED` event. It may restrict the content
   while reviewing.
3. The operator reviews the claim, the uploader's evidence and the source.
4. The node marks the claim `verified_evidence`, `disputed` or `invalidated`.
5. In Phase 2, the node may publish a signed moderation event.
6. Other nodes decide independently whether to follow it.

Do not automatically trust anonymous reports. Do not automatically reject
content because one node disputes it. Urgent reports still need a fast local
response.

## 6. Removal and tombstones

On removal:

- stop offering new downloads;
- remove from search;
- stop replication;
- notify known replicas where practical;
- keep a tombstone with the hash, event and reason;
- delete local bytes when legally required and technically possible.

Explain that a decentralised network cannot guarantee deletion of copies already
downloaded. Do not promise otherwise.

## 7. Illegal content

Illegal content is handled separately from copyright:

- restrict locally and immediately on credible notice;
- do not federate, replicate or retain samples for review;
- preserve only the minimum evidence required by law;
- follow specialist reporting and legal procedures;
- obtain legal advice for the operator's jurisdiction.

## 8. API and UI

- Public metadata must expose the licence status, not only the claim.
- A verified badge is allowed only for `verified_evidence`, with tooltip text
  explaining what was verified.
- Search results should show the licence label or "unverified".
- The upload form must not preselect a permissive licence.
- The upload response must state that the claim is unverified.
- Reports and disputes must be available without an account.

## 9. Node policy

Each node configures:

- accepted licence identifiers;
- whether `unverified` content is accepted;
- whether evidence is required for public listing;
- whether disputed content is restricted immediately;
- local blocklists and takedown contacts.

There is no central licence authority and no network-wide acceptance list.

## 10. Legal caution

This document is not legal advice. Before public deployment, the operator must
obtain advice on copyright, database rights, personality rights and illegal
content in its jurisdiction. The software must not imply that a licence claim is
legally verified when it is not.
