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

pub mod client;
pub mod error;
#[allow(non_camel_case_types)]
pub mod models;

pub use client::Client;
pub use error::Error;
pub use models::*;
