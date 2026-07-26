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
//! ignored, so a Cloud API change should not break a deployed consumer. Concretely, a
//! response that:
//!
//! * **adds** a field is accepted and the field ignored;
//! * **drops or renames** a field degrades to that field's default instead of failing
//!   the whole call;
//! * carries an **unrecognized enum value or union shape** lands in that type's
//!   `Unknown` catch-all, which holds the value verbatim and re-serializes losslessly.
//!
//! The residual case is a field whose *type* changes — an array that becomes a string,
//! say. `#[serde(default)]` fills in a missing key and cannot help there, so such a
//! change still fails the response for the model that holds the field. Enums and
//! discriminated unions absorb it into `Unknown`; a plain struct field does not.
//!
//! Requests stay strict through the type system: fields the API requires are `T`, not
//! `Option<T>`, and the defaults never change what is serialized.
//!
//! The caveat is read-modify-write. A consumer that `GET`s an object, changes one field,
//! and writes the whole thing back can persist a defaulted value — an empty string, say —
//! for a field the server stopped sending. Callers doing read-modify-write should send an
//! explicit set of the fields they mean to change rather than echoing back a deserialized
//! response. `clickhousectl` itself does not round-trip: request types are separate from
//! response models and its update commands build bodies from CLI flags.
//!
//! Exposure to a silently defaulted field, and to the residual type-change case, is
//! bounded by the daily OpenAPI drift job, which compares the live spec against these
//! models and files an issue when they diverge.

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
