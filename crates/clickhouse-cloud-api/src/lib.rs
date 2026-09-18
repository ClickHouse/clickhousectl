//! # clickhouse-cloud-api
//!
//! Typed Rust client for the ClickHouse Cloud API.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use clickhouse_cloud_api::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), clickhouse_cloud_api::Error> {
//!     let client = Client::new("your-key-id", "your-key-secret");
//!     let orgs = client.organization_get_list().await?;
//!     println!("{:?}", orgs);
//!     Ok(())
//! }
//! ```
//!
//! ## Request and response models
//!
//! Requests are strict and responses are tolerant, and both are expressed in the
//! type system rather than through serde attributes.
//!
//! A request model mirrors the spec: a required field is `T`, an optional or
//! nullable field is `Option<T>`. **Every field of every response model is
//! `Option<T>`**, so a field the API stops sending, or sends as `null`,
//! deserializes to `None` instead of failing the whole response. Several teams
//! evolve the Cloud API independently; a strict response field would make each of
//! those changes a breaking one for you. Nothing is fabricated to fill a gap, so
//! "the server sent `0`" and "the server dropped the field" stay distinguishable,
//! and absence is resolved where the value is used.
//!
//! A schema the API uses in both directions is therefore two Rust types: the
//! request variant keeps the schema's name and the response variant is
//! `{Name}Response`, as in [`models::PostgresInstanceConfig`] and
//! [`models::PostgresInstanceConfigResponse`]. Response models implement
//! [`serde::Serialize`] with absent fields **omitted**, never written as `null`,
//! so serializing one reproduces the key set the API sent.
//!
//! Editing a fetched resource and writing it back crosses that boundary
//! deliberately. [`convert`] holds the conversions, and a fallible one names the
//! wire fields it needs via [`MissingRequiredFields`]:
//!
//! ```rust,no_run
//! use clickhouse_cloud_api::{Client, PostgresInstanceConfig};
//!
//! # async fn write_back() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new("your-key-id", "your-key-secret");
//! let fetched = client
//!     .postgres_instance_config_get("org-id", "postgres-id")
//!     .await?
//!     .result
//!     .ok_or("the API returned no result")?;
//!
//! // Absence has to be resolved before the value can become a write body.
//! let mut body = PostgresInstanceConfig::try_from(fetched)?;
//! body.pg_config.autovacuum_max_workers = Some(4.into());
//! client
//!     .postgres_instance_config_post("org-id", "postgres-id", &body)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! Enums and unions are tolerant in their own right, in both directions: a string
//! enum keeps an `Unknown(String)` catch-all and an object union an
//! `Unknown(serde_json::Value)` fallback holding the payload verbatim, so an
//! unrecognized or reshaped variant round-trips rather than being rejected.
//! Unknown object fields are ignored everywhere.
//!
//! One failure mode remains, honestly: a field that is *present* with a different
//! type than the spec declares. `Option<T>` absorbs absence and `null`, not an
//! object where a string used to be, so such a change still fails a plain struct
//! field. Enums and unions absorb it through the catch-alls above. Detecting
//! spec drift of that kind is the job of the repository's daily OpenAPI drift
//! check, not of runtime deserialization.

pub mod client;
pub mod convert;
pub mod error;
pub mod meta;
#[allow(non_camel_case_types)]
pub mod models;
pub mod serde_helpers;

pub use client::Client;
pub use convert::MissingRequiredFields;
pub use error::Error;
pub use meta::{BETA_OPERATIONS, DEPRECATED_FIELDS, is_beta_operation, is_deprecated_field};
pub use models::*;
