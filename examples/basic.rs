//! Basic usage example — mirrors the Go SDK's ExampleClient.
//!
//! Set LOOPENGINE_PROJECT_KEY, LOOPENGINE_PROJECT_SECRET, LOOPENGINE_PROJECT_ID in your
//! environment and run with:
//!
//!   cargo run --example basic

use loopengine::Client;

#[tokio::main]
async fn main() {
    let key = std::env::var("LOOPENGINE_PROJECT_KEY").expect("LOOPENGINE_PROJECT_KEY not set");
    let secret =
        std::env::var("LOOPENGINE_PROJECT_SECRET").expect("LOOPENGINE_PROJECT_SECRET not set");
    let project_id =
        std::env::var("LOOPENGINE_PROJECT_ID").expect("LOOPENGINE_PROJECT_ID not set");

    let client = Client::new(key, secret, project_id).expect("Failed to build client");

    client
        .send(serde_json::json!({"message": "Hello from the Rust SDK"}))
        .await
        .expect("Failed to send feedback");

    println!("ok");
}
