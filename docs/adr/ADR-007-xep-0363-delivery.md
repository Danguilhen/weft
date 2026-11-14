# ADR-007 — XMPP clients send through XEP-0363

- Status: Accepted
- Date: 2026-11-14
- Affects: client integration, privacy, operations

## Context

Jabu needs to send a selected GIF into an XMPP conversation. It can either send
the Weft node URL directly or upload the bytes through the user's XMPP HTTP File
Upload service (XEP-0363). Direct links expose the node URL, can break when the
node removes the content, and make the recipient depend on that node. Uploading
copies the bytes and consumes the user's upload quota.

## Decision

- Default: fetch the selected asset from the Weft node, verify its SHA-256, upload
  it through XEP-0363, and send a file message with the uploaded URL and MIME.
- Keep a `link` send mode as an explicit setting for users who prefer not to
  duplicate the bytes.
- Never re-encode the GIF.
- Do not add tracking parameters to either URL.
- Include attribution and licence metadata where the message format supports it;
  otherwise use a readable fallback.
- Document that OMEMO encrypts the stanza, not the uploaded file.

## Consequences

- Positive: the recipient can retrieve the file even if the Weft node goes
  offline or removes it.
- Positive: the Weft node does not become a permanent hotlink dependency.
- Negative: download and upload bandwidth are used twice.
- Negative: the file is public at the XEP-0363 URL; this is an existing XMPP
  media limitation, not new to Weft.
- Follow-up: evaluate encrypted media sharing (for example XEP-0454) after the
  first integration.

## Alternatives considered

| Alternative | Why rejected as default |
|---|---|
| Send the Weft URL | Link rot and node dependency |
| Inline base64 in the message | Size limits and poor interop |
| P2P transfer | Not available in the XMPP file flow |
| Encrypted media first | Larger scope; keep as a later enhancement |

## References

- [`../XMPP.md`](../XMPP.md)
- [`../PROTOCOL.md`](../PROTOCOL.md)
