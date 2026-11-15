// SPDX-License-Identifier: MIT

//! A typed, asynchronous client for the public Weft node API.
//!
//! The client only depends on the public HTTP contract. It does not link the
//! server and can be used by any application, including non-Rust clients via
//! the same JSON shapes.
//!
//! ```no_run
//! # async fn example() -> Result<(), weft_client::Error> {
//! use weft_client::{Client, ListQuery};
//!
//! let client = Client::new("https://gifs.example")?;
//! let page = client.search(ListQuery::text("cat")).await?;
//! for item in page.items {
//!     println!("{} {}", item.title, item.media_url);
//! }
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod model;

pub use client::{discover, Client, ClientBuilder, DownloadedMedia, ListQuery, Sort, Window};
pub use error::Error;
pub use model::{
    Capabilities, ContentSummary, DiscoveryDocument, Licence, Manifest, ManifestLicence,
    Moderation, NodeDescriptor, Page, Source, Sources, TagCount,
};
