//! Minimal, high-performance client for the [LoopEngine](https://loopengine.dev) Ingest API.
//!
//! Create a [`Client`] with your project credentials, then call [`Client::send`] with your payload.
//! All signing and HTTP details are handled inside the crate. The client is safe for concurrent
//! use (it wraps a [`reqwest::Client`] which uses a connection pool internally).
//!
//! # Quick start
//!
//! ```rust,no_run
//! use loopengine::Client;
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::new("project_key", "project_secret", "project_id").unwrap();
//!     client
//!         .send(serde_json::json!({"message": "user feedback"}))
//!         .await
//!         .unwrap();
//! }
//! ```

mod client;
mod error;
mod sign;

pub use client::{Client, ClientBuilder};
pub use error::Error;
