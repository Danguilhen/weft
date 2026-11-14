# Weft — XMPP client integration (Jabu)

Status: design for Milestone 6. The MVP and federation do not depend on Jabu.

## 1. Goals

Jabu should offer a GIF and sticker picker backed by a Weft node, without a
central account or a commercial provider.

The client must:

- search and browse a selected Weft node;
- preview animated media;
- show attribution and licence status;
- send the chosen media into a 1:1 chat or a room;
- work with a self-hosted node or a user-selected public node;
- send no tracking parameters and no XMPP JID to the node.

## 2. Architecture

```text
Jabu UI (Dioxus)
   │
   ├── GifProvider trait ── WeftProvider ── weft-client ── HTTPS ── Weft node
   │
   └── XMPP file upload (XEP-0363) ── message with file metadata
```

- `weft-client` is the MIT-licensed Rust client crate from the Weft repository.
- Jabu defines a provider trait so another backend can be added later.
- The Weft node is selected in settings; there is no built-in central node.
- Fetching a GIF is a separate step from sending it.

## 3. Settings

```toml
[gif]
enabled = true
# Node origin, not an account. Empty disables the picker.
node_url = "https://gifs.example"
# Allow the user to enter a different node.
allow_custom_node = true
# Rating filter sent to the node when supported.
rating = "pg"              # g | pg | pg-13
# Start previews automatically; reduce motion still wins.
autoplay = true
# upload = fetch and upload via XEP-0363; link = send the node URL.
send_mode = "upload"
```

UI settings should also expose node URL, autoplay and send mode. Do not ship a
default commercial provider.

## 4. Provider trait

A shape similar to:

```rust
#[async_trait]
pub trait GifProvider: Send + Sync {
    async fn search(&self, query: &str, cursor: Option<String>, limit: u32)
        -> Result<GifPage, GifError>;
    async fn trending(&self, cursor: Option<String>, limit: u32)
        -> Result<GifPage, GifError>;
    async fn tags(&self) -> Result<Vec<GifTag>, GifError>;
    async fn fetch(&self, content_id: &str) -> Result<GifAsset, GifError>;
}

pub struct GifPage {
    pub items: Vec<GifSummary>,
    pub next_cursor: Option<String>,
}

pub struct GifSummary {
    pub content_id: String,
    pub media_url: String,
    pub preview_url: String,
    pub width: u32,
    pub height: u32,
    pub mime: String,
    pub title: Option<String>,
    pub licence: Option<LicenceView>,
    pub attribution: Option<String>,
}

pub struct GifAsset {
    pub bytes: Vec<u8>,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    pub licence: Option<LicenceView>,
    pub attribution: Option<String>,
}
```

The real trait belongs in Jabu; `weft-client` exposes the HTTP types.

## 5. Picker UX

- A GIF button next to emoji and stickers.
- Tabs or sections for Trending, Search and Tags.
- A search field with debounce and cancellation.
- A grid of previews, keyboard navigable, with `aria-label` from the title.
- A detail/preview state showing the animation, dimensions, licence and
  attribution.
- Loading, empty, offline, rate-limited and error states.
- No infinite scroll without a stop; use cursor pagination and a hard page cap.
- Respect reduced motion: do not autoplay previews when the OS asks to reduce
  motion.
- Show "Unverified licence" when the status is not `verified_evidence`.

## 6. Send flow

```text
user selects GIF
  → fetch manifest and bytes from the node
  → verify SHA-256 and size
  → upload bytes via XEP-0363 (upload = default)
  → send a file message with the uploaded URL, MIME and dimensions
  → include attribution/licence in the message metadata where supported
```

Rules:

- Verify the hash before uploading; do not send bytes the node cannot vouch for.
- Preserve the original bytes; do not re-encode a GIF.
- Use a neutral filename derived from the hash, not a tracking URL.
- Do not add the node URL as a tracking parameter; a readable attribution is a
  separate text/metadata field.
- If `send_mode = "link"`, send the Weft media URL directly and state that it may
  expire if the node removes the content.
- Rooms use the same XEP-0363 path as 1:1 chats.
- Respect the node's size limits and the XMPP upload service's limits.
- On upload failure, keep the picker open and show a retry; do not send a broken
  link.

### OMEMO caveat

OMEMO encrypts the message body and the file URL in the stanza, but the uploaded
file itself is public at the XEP-0363 URL. This is the same limitation as any
existing file upload. Do not claim end-to-end encrypted media. Encrypted media
sharing (for example XEP-0454 or an encrypted container) is out of scope for the
first integration; document the limitation.

## 7. Privacy

- The Weft node sees the client IP for search, manifest and media requests. Use
  a node the user trusts, or a future proxy.
- Do not send the user's JID, display name or device ID to the node.
- Do not add analytics, referrers or provider keys.
- Do not cache search queries longer than needed; do not log them in the UI.
- The XEP-0363 upload server already sees the user's address; the Weft node sees
  only the fetch.
- A link-mode send exposes the node URL to the recipient; upload mode exposes the
  XMPP upload URL instead.

## 8. Animated attachment fix (independent)

Before or alongside the picker, fix Jabu's existing attachment rendering:

- For `image/gif`, animated WebP and APNG, prefer the original URL over the
  static `thumbnail_url`.
- Use the thumbnail as a placeholder until the original loads.
- Add a small GIF/animated badge.
- Avoid cropping tall media (`object-cover` with a hard max height); use a
  contained layout with a sensible maximum.
- Add a unit test for the display-URL choice.
- This change does not depend on Weft and can ship immediately.

## 9. Errors and limits

Map node errors to user-facing messages:

| Node error | UI |
|---|---|
| `rate_limited` | "Too many requests; try again in N seconds." |
| `content_not_found` / `content_removed` | "This GIF is no longer available." |
| `validation_failed` | "This GIF cannot be sent: <safe reason>." |
| Network/offline | "Cannot reach the GIF node." |
| Upload failure | "The GIF could not be uploaded; retry." |

Do not expose raw server errors, stack traces or node URLs in a way that leaks
private infrastructure.

## 10. Tests

- Mock provider: search, pagination, empty, rate limit, invalid manifest.
- Hash verification rejects mismatched bytes.
- Upload mode calls XEP-0363 with the correct MIME and filename.
- Link mode sends the node URL without tracking parameters.
- No JID or display name is sent to the provider.
- Reduced-motion disables autoplay.
- The picker is keyboard navigable and has accessible labels.
- Animated attachment rendering prefers the original for GIF/WebP/APNG.

## 11. Phases

1. Animated attachment fix in Jabu (no Weft dependency).
2. `weft-client` API and a mock provider.
3. Picker UI against a local Weft node.
4. XEP-0363 send flow in 1:1 and rooms.
5. Settings, attribution/licence display and error states.
6. Optional: user-selected node discovery and a local node shortcut.
