# weft-client

Typed, asynchronous Rust client for the public Weft node API. The crate is
**MIT** licensed, does not link the server, and only depends on the HTTP
contract.

```toml
[dependencies]
weft-client = { git = "https://github.com/Danguilhen/weft", package = "weft-client" }
```

```rust,no_run
use weft_client::{Client, ListQuery};

# async fn run() -> Result<(), weft_client::Error> {
let client = Client::new("https://gifs.example")?;
let page = client.search(ListQuery::text("cat")).await?;
for item in page.items {
    println!("{}: {}", item.title, item.media_url);
    let bytes = client.download(&item.hash).await?;
    // Hand the bytes to an XEP-0363 upload, a cache or a renderer.
}
# Ok(())
# }
```

## Covered endpoints

- `GET /api/v1/node`
- `GET /api/v1/search`
- `GET /api/v1/trending`
- `GET /api/v1/media`
- `GET /api/v1/tags`
- `GET /api/v1/media/{hash}` and `/sources`
- `GET /media/{hash}`

Admin and upload endpoints are intentionally not exposed: node operators keep
the admin token server-side, and clients that upload should use the XEP-0363
flow documented in [`docs/XMPP.md`](../../docs/XMPP.md).

## Behaviour

- Base URLs may include a path prefix.
- List limits are clamped to `1..=100`; cursors stay opaque.
- API errors preserve the node error code and `Retry-After`.
- Content hashes are validated locally before a request.
- `download` returns the original bytes plus `Content-Type`, `ETag` and
  `Digest` for verification.
- MSRV is 1.70 so clients on older toolchains can embed it.
