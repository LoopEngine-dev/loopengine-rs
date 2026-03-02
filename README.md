# LoopEngine Rust SDK

Rust client for the [LoopEngine](https://loopengine.dev) Ingest API. Create a client with your credentials, then call `send` with your payload.

**Requirements:** Rust 1.75+ (async/await, `impl Trait`)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
loopengine = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## Usage

```rust
use loopengine::Client;

#[tokio::main]
async fn main() {
    let client = Client::new(
        std::env::var("LOOPENGINE_PROJECT_KEY").unwrap(),
        std::env::var("LOOPENGINE_PROJECT_SECRET").unwrap(),
        std::env::var("LOOPENGINE_PROJECT_ID").unwrap(),
    )
    .unwrap();

    client
        .send(serde_json::json!({"message": "User feedback here"}))
        .await
        .unwrap();
}
```

- **`Client::new(key, secret, project_id)`** — Builds a client. Use your project key, secret, and project ID from the LoopEngine dashboard. Returns an error if any credential is empty.
- **`client.send(payload).await`** — Sends the payload to the Ingest API at `api.loopengine.dev`. `project_id` is added automatically. The payload must match the **fields and constraints** configured for your project in the LoopEngine dashboard (e.g. required fields, allowed keys, value types). You can pass any value that implements `serde::Serialize` and serializes to a JSON object (a `serde_json::json!` map, a struct with `#[derive(Serialize)]`, etc.). Use **`client.send_with_geo(payload, lat, lon).await`** to send device coordinates (see Geolocation below).

The client is safe for concurrent use — it wraps a [`reqwest::Client`](https://docs.rs/reqwest) which maintains an internal connection pool.

## Geolocation

You can send device location so feedback is associated with coordinates instead of IP-based geo. Use `send_with_geo` and pass `Some(lat)` and `Some(lon)`. When **both** are provided, the SDK adds `geo_lat` and `geo_lon` to the request body; they are included in the HMAC signature. Pass `None` for both to use IP-based geolocation (or use `send(payload)`). Valid ranges: latitude -90 to 90, longitude -180 to 180.

```rust
// Without geo (IP-based location is used)
client.send(serde_json::json!({"message": "Feedback"})).await?;

// With device coordinates
client
    .send_with_geo(
        serde_json::json!({"message": "Bug at my location"}),
        Some(34.05),
        Some(-118.25),
    )
    .await?;
```

## Custom HTTP client

Use `Client::builder` to pass a custom `reqwest::Client` (e.g. to set timeouts or a custom connector):

```rust
use std::time::Duration;

let http = reqwest::Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .unwrap();

let client = loopengine::Client::builder(key, secret, project_id)
    .with_http_client(http)
    .build()
    .unwrap();
```

## Error handling

All errors are represented by `loopengine::Error`:

| Variant | When |
|---|---|
| `MissingCredentials` | A credential is empty after trimming |
| `Serialize` | Payload could not be serialized to JSON |
| `Http` | Network / transport error |
| `ApiError { status, body }` | Server returned a non-2xx response |

## Request signing

Every request is signed with `HMAC-SHA256` over the canonical string `"METHOD\nPATH\nTIMESTAMP\nSHA256(body)"`. The signature is base64url-encoded (no padding) and sent as the `X-Signature: v1=<sig>` header alongside `X-Project-Key` and `X-Timestamp`. All signing logic is handled transparently by the SDK.

## License

MIT
