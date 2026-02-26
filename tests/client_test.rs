use loopengine::Client;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Start a mock server and return a client pointing at it.
async fn make_client(project_id: &str) -> (Client, MockServer) {
    let server = MockServer::start().await;
    let client = Client::builder("pk_test", "psk_test", project_id)
        .with_base_url(server.uri())
        .build()
        .unwrap();
    (client, server)
}

#[tokio::test]
async fn send_injects_project_id() {
    let (client, server) = make_client("proj_123").await;

    Mock::given(method("POST"))
        .and(path("/feedback"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    client
        .send(serde_json::json!({"message": "hello"}))
        .await
        .unwrap();

    // Verify the mock was hit exactly once.
    server.verify().await;
}

#[tokio::test]
async fn send_returns_error_on_4xx() {
    let (client, server) = make_client("proj_err").await;

    Mock::given(method("POST"))
        .and(path("/feedback"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string(r#"{"error":"bad key"}"#),
        )
        .mount(&server)
        .await;

    let err = client
        .send(serde_json::json!({"message": "x"}))
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("401"), "error should contain status, got: {msg}");
}

#[tokio::test]
async fn send_returns_error_on_5xx() {
    let (client, server) = make_client("proj_5xx").await;

    Mock::given(method("POST"))
        .and(path("/feedback"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let err = client
        .send(serde_json::json!({"data": "test"}))
        .await
        .unwrap_err();

    assert!(err.to_string().contains("500"));
}

#[tokio::test]
async fn send_sets_required_headers() {
    use wiremock::matchers::header_exists;

    let (client, server) = make_client("proj_headers").await;

    Mock::given(method("POST"))
        .and(path("/feedback"))
        .and(header_exists("X-Project-Key"))
        .and(header_exists("X-Timestamp"))
        .and(header_exists("X-Signature"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    client
        .send(serde_json::json!({"msg": "check headers"}))
        .await
        .unwrap();

    server.verify().await;
}

#[tokio::test]
async fn new_rejects_missing_credentials() {
    assert!(Client::new("", "psk", "proj").is_err());
    assert!(Client::new("pk", "", "proj").is_err());
    assert!(Client::new("pk", "psk", "").is_err());
}
