//! Error types for the ClickHouse Cloud API client.

/// Errors that can occur when using the client.
///
/// `#[non_exhaustive]`: a new failure mode a caller must tell apart gets its
/// own variant (see `Error::Sql`), and that must not be a breaking change for
/// every downstream `match`. Callers keep a `_` arm and treat what they do not
/// recognise as an unclassified error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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

    /// The Query API reported a ClickHouse SQL-level error: the request
    /// reached the service, which rejected the statement itself and answered
    /// with a `{"error": {"code": …, "details": …}}` body.
    ///
    /// Split out from [`Error::Api`] so callers can tell "the server refused
    /// this SQL" from "the request never got that far" *structurally* — the
    /// previous shape forced them to sniff the formatted message for a
    /// `SQL error ` prefix. The `Display` text is unchanged, so user-facing
    /// output is identical either way.
    #[error("SQL error {code}: {details}")]
    Sql {
        status: u16,
        /// ClickHouse error code as the query host reported it (numeric codes
        /// arrive as numbers, named ones as strings, so it stays a string).
        code: String,
        details: String,
    },

    /// Operation requires a different auth mode than the client was configured with.
    #[error("auth mismatch: {0}")]
    AuthMismatch(String),

    /// The Query API reported the service is idled and asked for an explicit
    /// wake confirmation (HTTP 206 `Confirm wake service`). Retry the query
    /// with `wake_service` set to wake the service and run it.
    #[error("service is idle; retry the query with wake_service to wake it")]
    ServiceIdle,

    /// The Query API reported the service is stopped (HTTP 206 `Service is
    /// stopped`, or its HTTP 404 unavailable-service response). A stopped
    /// service is never woken by the Query API; it must be started explicitly.
    #[error("service is stopped; it must be started before it can be queried")]
    ServiceStopped,

    /// The Query API gateway stopped waiting for the statement (HTTP 500 with
    /// the body `{"error": "Timeout error."}`).
    ///
    /// Only the HTTP response is lost: the statement keeps running on the
    /// service. That is why this is a variant of its own rather than an
    /// [`Error::Api`] a caller would have to recognise by its status — a
    /// caller that treats it as a transient 500 and resends the request runs
    /// the statement twice, which for an `INSERT` means loading the data
    /// twice.
    #[error(
        "the query timed out at the Query API gateway; the statement may still be running on the service"
    )]
    QueryTimeout,
}
