// SPDX-License-Identifier: MIT

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use weft_client::{Client, ListQuery, Sort, Window};

const SUMMARY: &str = r#"{
  "hash": "3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7b",
  "contentId": "urn:weft:sha256:3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7b",
  "mime": "image/gif",
  "width": 480,
  "height": 270,
  "frames": 42,
  "durationMs": 3200,
  "title": "Surprised cat",
  "tags": ["cat", "reaction"],
  "licence": { "spdx": "CC0-1.0", "status": "verified_evidence" },
  "mediaUrl": "https://gifs.example/media/3a7b",
  "manifestUrl": "https://gifs.example/api/v1/media/3a7b",
  "source": "https://example.org/original.gif",
  "createdAt": "2026-11-14T00:00:00Z",
  "status": "active"
}"#;

fn hash() -> &'static str {
    "3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7bd3e2360a3a7b"
}

async fn serve_once(
    status: &str,
    content_type: &str,
    extra_headers: &str,
    body: String,
) -> (String, oneshot::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    let status = status.to_string();
    let content_type = content_type.to_string();
    let extra_headers = extra_headers.to_string();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0_u8; 8192];
        let read = socket.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        let _ = tx.send(request);
    });
    (format!("http://{address}"), rx)
}

#[tokio::test]
async fn search_sends_the_query_and_parses_a_page() {
    let body = format!(r#"{{ "items": [{SUMMARY}], "nextCursor": "next" }}"#);
    let (base, request) = serve_once("200 OK", "application/json", "", body).await;
    let client = Client::new(&base).unwrap();
    let page = client
        .search(
            ListQuery::text("cat")
                .with_tag("reaction")
                .with_sort(Sort::Newest)
                .with_limit(5),
        )
        .await
        .unwrap();

    let request = request.await.unwrap();
    assert!(request.starts_with("GET /api/v1/search?"), "{request}");
    assert!(request.contains("q=cat"));
    assert!(request.contains("tags=reaction"));
    assert!(request.contains("sort=newest"));
    assert!(request.contains("limit=5"));
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].title, "Surprised cat");
    assert_eq!(page.items[0].licence.status, "verified_evidence");
    assert!(page.items[0].is_animated());
    assert_eq!(page.next_cursor.as_deref(), Some("next"));
}

#[tokio::test]
async fn trending_sends_its_window() {
    let (base, request) = serve_once(
        "200 OK",
        "application/json",
        "",
        r#"{"items":[],"nextCursor":null}"#.to_string(),
    )
    .await;
    let client = Client::new(&base).unwrap();
    client
        .trending(ListQuery::trending(Window::Week))
        .await
        .unwrap();
    let request = request.await.unwrap();
    assert!(request.starts_with("GET /api/v1/trending?"), "{request}");
    assert!(request.contains("sort=trending"));
    assert!(request.contains("window=7d"));
}

#[tokio::test]
async fn tags_are_returned_without_pagination() {
    let (base, _) = serve_once(
        "200 OK",
        "application/json",
        "",
        r#"{"items":[{"tag":"cat","count":12}],"nextCursor":null}"#.to_string(),
    )
    .await;
    let client = Client::new(&base).unwrap();
    let tags = client.tags().await.unwrap();
    assert_eq!(tags[0].tag, "cat");
    assert_eq!(tags[0].count, 12);
}

#[tokio::test]
async fn manifest_preserves_the_signature_and_evidence() {
    let body = format!(
        r#"{{
          "version": "weft-manifest/0.1",
          "contentId": "urn:weft:sha256:{hash}",
          "hash": "{hash}",
          "mime": "image/gif",
          "size": 955,
          "width": 80,
          "height": 60,
          "frames": 2,
          "durationMs": 200,
          "createdAt": "2026-11-14T00:00:00Z",
          "updatedAt": "2026-11-14T00:00:00Z",
          "origin": "https://gifs.example/actor",
          "source": "https://example.org/original.gif",
          "sources": [],
          "licence": {{
            "spdx": "CC0-1.0",
            "status": "verified_evidence",
            "evidence": ["https://example.org/licence"]
          }},
          "moderation": {{ "state": "active", "updatedAt": "2026-11-14T00:00:00Z" }},
          "tags": ["cat"],
          "signature": {{ "alg": "Ed25519", "keyId": "https://gifs.example/actor#key-1", "value": "abc" }}
        }}"#,
        hash = hash()
    );
    let (base, request) = serve_once("200 OK", "application/json", "", body).await;
    let client = Client::new(&base).unwrap();
    let manifest = client.manifest(hash()).await.unwrap();
    let request = request.await.unwrap();
    assert!(request.starts_with(&format!("GET /api/v1/media/{}", hash())));
    assert_eq!(manifest.signature.alg, "Ed25519");
    assert_eq!(manifest.licence.evidence.len(), 1);
    assert_eq!(manifest.moderation.state, "active");
}

#[tokio::test]
async fn api_errors_keep_the_code_and_retry_after() {
    let (base, _) = serve_once(
        "429 Too Many Requests",
        "application/json",
        "Retry-After: 7\r\n",
        r#"{"error":{"code":"rate_limited","message":"Too many uploads; retry later."}}"#
            .to_string(),
    )
    .await;
    let client = Client::new(&base).unwrap();
    let error = client.search(ListQuery::new()).await.unwrap_err();
    assert_eq!(error.status(), Some(429));
    assert_eq!(error.retry_after(), Some(7));
    assert!(error.is_rate_limited());
    assert!(error.to_string().contains("rate_limited"));
}

#[tokio::test]
async fn download_returns_the_original_bytes_and_headers() {
    let (base, request) = serve_once(
        "200 OK",
        "image/gif",
        "ETag: \"sha256:abc\"\r\nDigest: sha-256=:abc=:\r\n",
        "GIF89a-demo".to_string(),
    )
    .await;
    let client = Client::new(&base).unwrap();
    let media = client.download(hash()).await.unwrap();
    let request = request.await.unwrap();
    assert!(request.starts_with(&format!("GET /media/{}", hash())));
    assert_eq!(media.bytes, b"GIF89a-demo");
    assert_eq!(media.content_type.as_deref(), Some("image/gif"));
    assert_eq!(media.etag.as_deref(), Some("\"sha256:abc\""));
    assert_eq!(media.digest.as_deref(), Some("sha-256=:abc=:"));
}

#[tokio::test]
async fn invalid_input_is_rejected_before_a_request() {
    assert!(matches!(
        Client::new("ftp://gifs.example"),
        Err(weft_client::Error::UnsupportedScheme)
    ));
    assert!(matches!(
        Client::new("not a url"),
        Err(weft_client::Error::InvalidBaseUrl(_))
    ));
    let client = Client::new("https://gifs.example").unwrap();
    assert!(matches!(
        client.manifest("not-a-hash").await,
        Err(weft_client::Error::InvalidHash(_))
    ));
}

#[tokio::test]
async fn a_base_path_is_preserved() {
    let (base, request) = serve_once(
        "200 OK",
        "application/json",
        "",
        r#"{"version":"0.1.0","nodeId":"https://gifs.example/actor","name":"Weft","description":"","publicUrl":"https://gifs.example","capabilities":{"api":["v1"],"media":["weft-wmp/0.1"],"replication":false,"federation":false}}"#.to_string(),
    )
    .await;
    let base = format!("{base}/weft/");
    let client = Client::new(base).unwrap();
    let node = client.node().await.unwrap();
    let request = request.await.unwrap();
    assert!(request.starts_with("GET /weft/api/v1/node"), "{request}");
    assert_eq!(node.capabilities.media[0], "weft-wmp/0.1");
}

const DISCOVERY: &str = r#"{
  "version": "weft-client/0.1",
  "node": "https://gifs.example",
  "name": "Example Weft",
  "api": "https://gifs.example/api/v1",
  "protocol": "weft-wmp/0.1"
}"#;

#[tokio::test]
async fn discovery_reads_the_well_known_document() {
    let (base, request) = serve_once("200 OK", "application/json", "", DISCOVERY.to_string()).await;
    let document = weft_client::discover(&base).await.unwrap();
    let request = request.await.unwrap();
    assert!(
        request.starts_with("GET /.well-known/weft-client"),
        "{request}"
    );
    assert_eq!(document.node, "https://gifs.example");
    assert_eq!(document.name.as_deref(), Some("Example Weft"));
    assert_eq!(document.api.as_deref(), Some("https://gifs.example/api/v1"));
}

#[tokio::test]
async fn discovery_rejects_an_insecure_node() {
    let body = r#"{"version":"weft-client/0.1","node":"http://gifs.example"}"#;
    let (base, _) = serve_once("200 OK", "application/json", "", body.to_string()).await;
    let error = weft_client::discover(&base).await.unwrap_err();
    assert!(matches!(error, weft_client::Error::InvalidDiscovery(_)));
}

#[tokio::test]
async fn discovery_rejects_an_unknown_version() {
    let body = r#"{"version":"other/1","node":"https://gifs.example"}"#;
    let (base, _) = serve_once("200 OK", "application/json", "", body.to_string()).await;
    let error = weft_client::discover(&base).await.unwrap_err();
    assert!(matches!(error, weft_client::Error::InvalidDiscovery(_)));
}
