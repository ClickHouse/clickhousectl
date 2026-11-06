mod activity;
mod api_keys;
mod backups;
mod byoc;
mod clickpipes;
mod clickstack;
mod cross_domain;
mod organizations;
mod postgres;
mod query_api_endpoints;
mod saved_queries;
mod scim;
mod services;
mod shared;
mod udfs;
mod whoami;

/// Shared assertion for the discriminated-union `Unknown` catch-all: an
/// unrecognized payload must deserialize into the union's lossless `Unknown`
/// variant (confirmed by `is_unknown`) and re-serialize to the byte-identical
/// JSON object it came from.
fn assert_unknown_variant_round_trips<T>(json: &str, is_unknown: impl FnOnce(&T) -> bool)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let original: serde_json::Value = serde_json::from_str(json).unwrap();
    let parsed: T = serde_json::from_str(json).unwrap();
    assert!(
        is_unknown(&parsed),
        "payload did not deserialize to the Unknown variant"
    );
    let reserialized = serde_json::to_value(&parsed).unwrap();
    assert_eq!(reserialized, original);
    assert!(reserialized.is_object());
}
