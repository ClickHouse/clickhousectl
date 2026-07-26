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
//! ## Tolerant deserialization
//!
//! Every field on every model carries `#[serde(default)]`, and unknown fields are
//! ignored. A response that drops, renames, or changes a field therefore degrades to
//! that field's default instead of failing the whole call, so a Cloud API change cannot
//! break a deployed consumer. Requests stay strict through the type system: fields the
//! API requires are `T`, not `Option<T>`, and the defaults never change what is
//! serialized.
//!
//! The caveat is read-modify-write. A consumer that `GET`s an object, changes one field,
//! and writes the whole thing back can persist a defaulted value — an empty string, say —
//! for a field the server stopped sending. Callers doing read-modify-write should send an
//! explicit set of the fields they mean to change rather than echoing back a deserialized
//! response. `clickhousectl` itself does not round-trip: update commands build request
//! bodies from CLI flags, and the `Patch` request types are separate and all-optional.
//!
//! Exposure to a silently defaulted field is bounded by the daily OpenAPI drift job,
//! which compares the live spec against these models and files an issue when they
//! diverge.

pub mod client;
pub mod error;
pub mod meta;
#[allow(non_camel_case_types)]
pub mod models;
pub mod serde_helpers;

pub use client::Client;
pub use error::Error;
pub use meta::{BETA_OPERATIONS, DEPRECATED_FIELDS, is_beta_operation, is_deprecated_field};
pub use models::*;
