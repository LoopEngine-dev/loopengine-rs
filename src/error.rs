use thiserror::Error;

/// Errors returned by [`crate::Client`].
#[derive(Debug, Error)]
pub enum Error {
    /// One or more required credentials were empty.
    #[error("loopengine: project_key, project_secret, and project_id are required")]
    MissingCredentials,

    /// The payload could not be serialized to JSON.
    #[error("loopengine: serialize payload: {0}")]
    Serialize(#[from] serde_json::Error),

    /// An HTTP transport or connection error occurred.
    #[error("loopengine: request: {0}")]
    Http(#[from] reqwest::Error),

    /// The API returned a non-2xx status code.
    #[error("loopengine: {status} {body}")]
    ApiError {
        /// HTTP status code returned by the server.
        status: u16,
        /// Response body (may be empty).
        body: String,
    },
}
