pub(super) mod bigquery;
pub(super) mod kafka;
pub(super) mod kinesis;
pub(super) mod mongodb;
pub(super) mod mysql;
pub(super) mod object_storage;
pub(super) mod postgres;
pub(super) mod pubsub;

use crate::cloud::client::{
    CloudError, Result as CloudResult, read_flag_file, read_flag_file_bytes,
};
use clap::builder::{PossibleValue, TypedValueParser};

pub(super) const DB_AUTHS: &[&str] = &["basic", "IAM_ROLE"];
pub(super) const REPLICATION_MODES: &[&str] = &["cdc", "snapshot", "cdc_only"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GcpAuthentication {
    ServiceAccount,
    WorkloadIdentity,
}

/// Expose canonical choices while retaining the existing authentication parser.
#[derive(Clone)]
pub(super) struct AuthenticationValueParser {
    parse: fn(&str) -> Result<String, String>,
    values: &'static [&'static str],
}

impl TypedValueParser for AuthenticationValueParser {
    type Value = String;

    fn parse_ref(
        &self,
        command: &clap::Command,
        argument: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        (self.parse).parse_ref(command, argument, value)
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(
            self.values.iter().copied().map(PossibleValue::new),
        ))
    }
}

// Keep I/O failures typed; an empty schema is a usage error. The API enforces
// the encoded size limit.
pub(super) fn read_protobuf_schema_file(path: &str) -> CloudResult<String> {
    let contents = if path == "-" {
        use std::io::Read as _;
        let mut contents = Vec::new();
        std::io::stdin().read_to_end(&mut contents)?;
        contents
    } else {
        read_flag_file_bytes("--protobuf-schema-file", path)?
    };
    if contents.is_empty() {
        return Err(CloudError::usage(if path == "-" {
            "no Protobuf schema received on stdin".to_string()
        } else {
            format!("Protobuf schema file '{path}' was empty")
        }));
    }

    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        contents,
    ))
}

/// Read a GCP service-account JSON key file from disk and return the
/// base64-encoded contents. Used by the object-storage, BigQuery and Pub/Sub
/// `create` handlers — the upstream API wants the encoded blob regardless
/// of which source it ends up on.
///
/// A path of `-` reads the key from stdin, the same spelling
/// `service query --queries-file -` uses, so a key held in a secret manager
/// never has to be written to disk. An empty key is refused here rather than
/// sent as an empty string; neither that error nor the io error names anything
/// but the path, so the key itself cannot reach an error message.
pub(super) fn read_gcp_service_account_file(path: &str) -> CloudResult<String> {
    let contents = if path == "-" {
        use std::io::Read as _;
        let mut contents = String::new();
        std::io::stdin().read_to_string(&mut contents)?;
        contents
    } else {
        read_flag_file("--service-account-file", path)?
    };
    if contents.trim().is_empty() {
        return Err(CloudError::usage(if path == "-" {
            "no service account key received on stdin".to_string()
        } else {
            format!("service account key file '{path}' was empty")
        }));
    }
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        contents.as_bytes(),
    ))
}
