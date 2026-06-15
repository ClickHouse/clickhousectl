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

    /// Operation requires a different auth mode than the client was configured with.
    #[error("auth mismatch: {0}")]
    AuthMismatch(String),

    /// The Query API reported the service is idled and asked for an explicit
    /// wake confirmation (HTTP 206 `Confirm wake service`). Retry the query
    /// with `wake_service` set to wake the service and run it.
    #[error("service is idle; retry the query with wake_service to wake it")]
    ServiceIdle,

    /// The Query API reported the service is stopped (HTTP 206 `Service is
    /// stopped`). A stopped service is never woken by the Query API; it must
    /// be started explicitly.
    #[error("service is stopped; it must be started before it can be queried")]
    ServiceStopped,
}
