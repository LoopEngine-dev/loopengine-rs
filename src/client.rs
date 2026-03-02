use crate::error::Error;
use crate::sign::sign_request;
use serde::Serialize;
use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://api.loopengine.dev";
const API_PATH: &str = "/feedback";

/// Sends feedback to the LoopEngine Ingest API. Safe for concurrent use.
///
/// Build with [`Client::new`] or [`Client::builder`].
pub struct Client {
    project_key: String,
    project_secret: String,
    project_id: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl Client {
    /// Builds a `Client` from project credentials.
    ///
    /// Returns [`Error::MissingCredentials`] if any credential is empty after trimming.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let client = loopengine::Client::new("pk", "psk", "proj_id").unwrap();
    /// ```
    pub fn new(
        project_key: impl Into<String>,
        project_secret: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Result<Self, Error> {
        ClientBuilder::new(project_key, project_secret, project_id).build()
    }

    /// Returns a [`ClientBuilder`] for configuring additional options before building `Client`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// let http = reqwest::Client::builder()
    ///     .timeout(Duration::from_secs(10))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = loopengine::Client::builder("pk", "psk", "proj_id")
    ///     .with_http_client(http)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder(
        project_key: impl Into<String>,
        project_secret: impl Into<String>,
        project_id: impl Into<String>,
    ) -> ClientBuilder {
        ClientBuilder::new(project_key, project_secret, project_id)
    }

    /// Sends `payload` to the Ingest API. `project_id` is injected automatically.
    ///
    /// `payload` must be any value that implements [`serde::Serialize`] and can be represented as
    /// a JSON object (e.g. a `struct` with `#[derive(Serialize)]`, or a
    /// [`serde_json::json!`] map). The fields and values must match the schema configured for
    /// your project in the LoopEngine dashboard.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails, the HTTP request fails, or the API returns a
    /// non-2xx status code.
    pub async fn send<T: Serialize>(&self, payload: T) -> Result<(), Error> {
        self.send_with_geo(payload, None, None).await
    }

    /// Sends `payload` to the Ingest API with optional device coordinates.
    ///
    /// When both `lat` and `lon` are [`Some`], the SDK adds `geo_lat` and `geo_lon` to the
    /// request body; they are included in the HMAC signature. Pass [`None`] for both to use
    /// IP-based geolocation. Valid ranges: latitude -90 to 90, longitude -180 to 180.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails, the HTTP request fails, or the API returns a
    /// non-2xx status code.
    pub async fn send_with_geo<T: Serialize>(
        &self,
        payload: T,
        lat: Option<f64>,
        lon: Option<f64>,
    ) -> Result<(), Error> {
        let body = self.build_body_with_geo(payload, lat, lon)?;

        let (timestamp, signature) =
            sign_request(&self.project_secret, "POST", API_PATH, &body);

        let url = format!("{}{}", self.base_url, API_PATH);

        let resp = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Project-Key", &self.project_key)
            .header("X-Timestamp", &timestamp)
            .header("X-Signature", &signature)
            .body(body)
            .send()
            .await
            .map_err(Error::Http)?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            return Err(Error::ApiError {
                status: status.as_u16(),
                body: body_text.trim().to_string(),
            });
        }

        Ok(())
    }

    /// Serializes `payload` to JSON bytes, injecting `project_id` and optionally `geo_lat`/`geo_lon`.
    fn build_body_with_geo<T: Serialize>(
        &self,
        payload: T,
        lat: Option<f64>,
        lon: Option<f64>,
    ) -> Result<Vec<u8>, Error> {
        let mut value = serde_json::to_value(payload)?;

        match value {
            Value::Object(ref mut map) => {
                map.insert(
                    "project_id".to_string(),
                    Value::String(self.project_id.clone()),
                );
                if let (Some(lat), Some(lon)) = (lat, lon) {
                    map.insert("geo_lat".to_string(), serde_json::json!(lat));
                    map.insert("geo_lon".to_string(), serde_json::json!(lon));
                }
            }
            Value::Null => {
                let mut map = serde_json::Map::new();
                map.insert(
                    "project_id".to_string(),
                    Value::String(self.project_id.clone()),
                );
                if let (Some(lat), Some(lon)) = (lat, lon) {
                    map.insert("geo_lat".to_string(), serde_json::json!(lat));
                    map.insert("geo_lon".to_string(), serde_json::json!(lon));
                }
                value = Value::Object(map);
            }
            _ => {
                // Non-object, non-null values are passed through as-is; project_id cannot be
                // injected into scalars or arrays.
            }
        }

        Ok(serde_json::to_vec(&value)?)
    }
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Builder for [`Client`]. Obtain one via [`Client::builder`].
pub struct ClientBuilder {
    project_key: String,
    project_secret: String,
    project_id: String,
    base_url: String,
    http_client: Option<reqwest::Client>,
}

impl ClientBuilder {
    /// Creates a new builder with the given credentials.
    pub fn new(
        project_key: impl Into<String>,
        project_secret: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Self {
        Self {
            project_key: project_key.into().trim().to_string(),
            project_secret: project_secret.into().trim().to_string(),
            project_id: project_id.into().trim().to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            http_client: None,
        }
    }

    /// Sets a custom [`reqwest::Client`]. Use to configure timeouts or a custom connector.
    ///
    /// If not set, a default [`reqwest::Client`] is used.
    pub fn with_http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Overrides the base URL (default: `https://api.loopengine.dev`). Mainly useful in tests.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Consumes the builder and returns a [`Client`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingCredentials`] if any credential is empty.
    pub fn build(self) -> Result<Client, Error> {
        if self.project_key.is_empty()
            || self.project_secret.is_empty()
            || self.project_id.is_empty()
        {
            return Err(Error::MissingCredentials);
        }

        Ok(Client {
            project_key: self.project_key,
            project_secret: self.project_secret,
            project_id: self.project_id,
            base_url: self.base_url,
            http_client: self.http_client.unwrap_or_default(),
        })
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ok() {
        let c = Client::new("pk", "psk", "proj").unwrap();
        assert_eq!(c.project_key, "pk");
        assert_eq!(c.project_secret, "psk");
        assert_eq!(c.project_id, "proj");
    }

    #[test]
    fn new_missing_credentials() {
        assert!(Client::new("", "psk", "proj").is_err());
        assert!(Client::new("pk", "", "proj").is_err());
        assert!(Client::new("pk", "psk", "").is_err());
    }

    #[test]
    fn new_trims_whitespace() {
        let c = Client::new("  pk  ", "  psk  ", "  proj  ").unwrap();
        assert_eq!(c.project_key, "pk");
        assert_eq!(c.project_secret, "psk");
        assert_eq!(c.project_id, "proj");
    }

    #[test]
    fn build_body_injects_project_id() {
        let c = Client::new("pk", "psk", "proj_123").unwrap();
        let body = c
            .build_body_with_geo(serde_json::json!({"message": "hi"}), None, None)
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["project_id"], "proj_123");
        assert_eq!(v["message"], "hi");
    }

    #[test]
    fn build_body_null_payload() {
        let c = Client::new("pk", "psk", "proj_123").unwrap();
        let body = c
            .build_body_with_geo(serde_json::Value::Null, None, None)
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["project_id"], "proj_123");
    }

    #[test]
    fn build_body_with_geo_injects_lat_lon() {
        let c = Client::new("pk", "psk", "proj_123").unwrap();
        let body = c
            .build_body_with_geo(
                serde_json::json!({"message": "hi"}),
                Some(34.05),
                Some(-118.25),
            )
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["project_id"], "proj_123");
        assert_eq!(v["message"], "hi");
        assert_eq!(v["geo_lat"], 34.05);
        assert_eq!(v["geo_lon"], -118.25);
    }

    #[test]
    fn build_body_with_geo_omits_when_only_one() {
        let c = Client::new("pk", "psk", "proj_123").unwrap();
        let body = c
            .build_body_with_geo(serde_json::json!({"message": "hi"}), Some(34.05), None)
            .unwrap();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert!(v.get("geo_lat").is_none());
        assert!(v.get("geo_lon").is_none());
    }
}
