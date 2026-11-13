//! Cloud subprocess tests: each test runs the real `clickhousectl` binary against a local `wiremock`
//! server standing in for the ClickHouse Cloud API, then asserts on what reached the wire, the exit
//! code and the output streams. Only behaviour a handler alone reaches belongs here; see the
//! "Tests" section of AGENTS.md. One module per `src/cloud` domain, plus cross-cutting client
//! behaviour, all in one binary so link time does not grow:
//!
//!     cargo test -p clickhousectl --test cloud

mod support;

mod activity;
mod api_keys;
mod auth;
mod backups;
mod clickpipes;
mod clickstack;
mod client;
mod organizations;
mod permissions;
mod postgres;
mod query;
mod query_api_endpoints;
mod saved_queries;
mod services;
mod shared;
mod udfs;
