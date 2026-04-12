//! Error types for the ClickHouse Cloud API client.

/// Errors that can occur when using the client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP transport error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error response.
    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },
}
