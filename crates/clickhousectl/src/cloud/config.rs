use crate::cloud::client::{CloudError, Result as CloudResult, read_flag_file};
use crate::failure::{ApiFailure, FailureKind};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::io::Read as _;

/// User-facing label for a file-or-stdin configuration source.
pub(crate) fn config_source_label(config_file: &str) -> &str {
    if config_file == "-" {
        "stdin"
    } else {
        config_file
    }
}

/// Read a command's primary JSON request body, the `--file` document that
/// [`assert_primary_json_input`] pins, from a file or stdin.
///
/// A file that cannot be read is a runtime I/O failure (exit 1, code `io`);
/// a body that is not JSON is invalid input (a usage error, exit 2).
pub(crate) fn read_config_value(config_file: &str) -> CloudResult<Value> {
    read_flag_config_value("--file", config_file)
}

/// Read the JSON document `flag` names from a file or stdin; see
/// [`read_config_value`] for how each failure is reported.
pub(crate) fn read_flag_config_value(flag: &str, config_file: &str) -> CloudResult<Value> {
    let contents = if config_file == "-" {
        let mut contents = String::new();
        std::io::stdin()
            .read_to_string(&mut contents)
            .map_err(|error| {
                CloudError::new(format!("failed to read {flag} from stdin: {error}"))
                    .with_failure(ApiFailure::new(FailureKind::Io))
            })?;
        contents
    } else {
        read_flag_file(flag, config_file)?
    };

    serde_json::from_str(&contents).map_err(|error| {
        CloudError::usage(format!(
            "failed to parse config {} as JSON: {error}",
            config_source_label(config_file)
        ))
    })
}

/// Append one object key to a JSON path, in the `tiles[0].config` form the
/// CLI uses for request-body locations.
fn push_path_key(path: &mut String, key: &str) {
    if !path.is_empty() {
        path.push('.');
    }
    path.push_str(key);
}

fn push_path_index(path: &mut String, index: usize) {
    path.push_str(&format!("[{index}]"));
}

fn ignored_path(base: &str, path: &serde_ignored::Path<'_>) -> String {
    fn collect(path: &serde_ignored::Path<'_>, rendered: &mut String) {
        match path {
            serde_ignored::Path::Root => {}
            serde_ignored::Path::Seq { parent, index } => {
                collect(parent, rendered);
                push_path_index(rendered, *index);
            }
            serde_ignored::Path::Map { parent, key } => {
                collect(parent, rendered);
                push_path_key(rendered, key);
            }
            // These variants are Rust deserializer traversal details, not
            // components of the JSON field path.
            serde_ignored::Path::Some { parent }
            | serde_ignored::Path::NewtypeStruct { parent }
            | serde_ignored::Path::NewtypeVariant { parent } => collect(parent, rendered),
        }
    }

    let mut rendered = base.to_string();
    collect(path, &mut rendered);
    rendered
}

fn error_path(base: &str, path: &serde_path_to_error::Path) -> String {
    let mut rendered = base.to_string();
    for segment in path {
        match segment {
            serde_path_to_error::Segment::Seq { index } => push_path_index(&mut rendered, *index),
            serde_path_to_error::Segment::Map { key } => push_path_key(&mut rendered, key),
            serde_path_to_error::Segment::Enum { variant } => push_path_key(&mut rendered, variant),
            serde_path_to_error::Segment::Unknown => push_path_key(&mut rendered, "?"),
        }
    }
    rendered
}

/// The JSON value a Rust number type accepts, for serde's primitive visitors,
/// which describe themselves by the bare type name (`f64`, `u32`, ...).
/// Fixed-width types small enough to overflow in practice keep their range,
/// so an out-of-range value still says what fits.
fn json_number_shape(primitive: &str) -> Option<&'static str> {
    Some(match primitive {
        "f32" | "f64" => "a number",
        "i8" => "an integer from -128 to 127",
        "i16" => "an integer from -32768 to 32767",
        "i32" => "an integer from -2147483648 to 2147483647",
        "i64" | "i128" | "isize" => "an integer",
        "u8" => "an integer from 0 to 255",
        "u16" => "an integer from 0 to 65535",
        "u32" => "an integer from 0 to 4294967295",
        "u64" | "u128" | "usize" => "a non-negative integer",
        _ => return None,
    })
}

/// Rewrite an "expected ..." clause that starts with a primitive number
/// type (or serde's `a nonzero u32` form), returning the JSON wording and the
/// text after the type name.
fn json_primitive_shape(expected: &str) -> Option<(&'static str, &str)> {
    let (nonzero, expected) = match expected.strip_prefix("a nonzero ") {
        Some(rest) => (true, rest),
        None => (false, expected),
    };
    let end = expected
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(expected.len());
    let (primitive, rest) = expected.split_at(end);
    let shape = json_number_shape(primitive)?;
    let shape = match (nonzero, primitive.starts_with('u')) {
        (false, _) => shape,
        (true, true) => "a positive integer",
        (true, false) => "a nonzero integer",
    };
    Some((shape, rest))
}

/// serde's data-model names for JSON arrays and objects, as it prints them
/// for the value it got (`invalid type: map, ...`) and the JSON word for each.
const SERDE_CONTAINERS: &[(&str, &str)] = &[("sequence", "array"), ("map", "object")];

/// Name the JSON container where serde names its own, on the side of the
/// message that describes the user's value. A string value is always quoted,
/// so the bare word can never be text the user wrote.
fn with_json_value_names(reason: &str) -> String {
    for (serde_name, json_name) in SERDE_CONTAINERS {
        let prefix = format!("invalid type: {serde_name}, ");
        if let Some(rest) = reason.strip_prefix(&prefix) {
            return format!("invalid type: {json_name}, {rest}");
        }
    }
    reason.to_string()
}

/// The JSON shape for the "expected ..." clause of serde's sequence and map
/// visitors (`Vec`, `BTreeMap`, ...), which say `a sequence` and `a map`.
fn json_container_shape(expected: &str) -> Option<&'static str> {
    match expected {
        "a sequence" => Some("an array"),
        "a map" => Some("an object"),
        _ => None,
    }
}

/// Replace the Rust type name serde's visitors put in an "expected ..."
/// clause with the JSON shape the user has to supply.
///
/// Derived visitors describe themselves as `struct RoleCreateRequest`,
/// `enum Kind` and so on, number visitors as `f64` or `u32`, and collection
/// visitors as `a sequence` or `a map`. Those names belong to the library or
/// to serde's data model, not to the request body the user wrote, so they
/// never reach a message. Only the text after the last `expected ` is
/// rewritten, apart from serde's bare name for the value it got: anything
/// else can quote the user's own value.
fn without_rust_type_names(reason: &str) -> String {
    let reason = with_json_value_names(reason);
    let reason = reason.as_str();
    // Longest phrase first, so `struct variant` is not read as `struct`.
    const SHAPES: &[(&str, &str)] = &[
        ("internally tagged enum ", "an object"),
        ("adjacently tagged enum ", "an object"),
        ("untagged enum ", "an accepted value"),
        ("struct variant ", "an object"),
        ("tuple variant ", "an array"),
        ("tuple struct ", "an array"),
        ("unit struct ", "null"),
        ("struct ", "an object"),
        ("enum ", "an accepted value"),
    ];
    const UNTAGGED: &str = "data did not match any variant of untagged enum ";

    if reason.starts_with(UNTAGGED) {
        return "value does not match any accepted form".to_string();
    }
    let Some((head, expected)) = reason.rsplit_once("expected ") else {
        return reason.to_string();
    };
    if let Some(shape) = json_container_shape(expected) {
        return format!("{head}expected {shape}");
    }
    if let Some((shape, rest)) = json_primitive_shape(expected) {
        return format!("{head}expected {shape}{rest}");
    }
    let Some((phrase, shape)) = SHAPES
        .iter()
        .find(|(phrase, _)| expected.starts_with(phrase))
    else {
        return reason.to_string();
    };
    let name_and_rest = &expected[phrase.len()..];
    let rest = name_and_rest
        .trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == ':');
    if rest.len() == name_and_rest.len() {
        return reason.to_string();
    }
    // A struct only reports a length when it was handed an array.
    if *phrase == "struct " && head.starts_with("invalid length ") {
        return format!("invalid type: array, expected {shape}");
    }
    format!("{head}expected {shape}{rest}")
}

/// Strictly deserialize a raw request body into a library request type.
///
/// Published response models intentionally ignore unknown fields. CLI request
/// input needs the opposite policy, so this wrapper reports every field serde
/// ignored, including nested fields, without changing the library's models.
pub(crate) fn deserialize_strict_config<T>(value: Value, source: &str) -> CloudResult<T>
where
    T: DeserializeOwned,
{
    deserialize_strict_value(value, "").map_err(|detail| {
        CloudError::usage(format!("invalid request body in config {source}: {detail}"))
    })
}

/// The check behind [`deserialize_strict_config`], returning the bare reason
/// on failure so a caller checking a nested value can report it under its own
/// path without repeating the config prefix.
///
/// `base` is the JSON path of `value` within the request body (empty for the
/// whole body); every reported location is a path below it. The value is
/// deserialized directly rather than re-encoded, so no message carries a line
/// or column that would point into text the user never wrote.
pub(crate) fn deserialize_strict_value<T>(value: Value, base: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let mut ignored = Vec::new();
    let mut record_ignored = |path: serde_ignored::Path<'_>| {
        ignored.push(ignored_path(base, &path));
    };
    let request = serde_path_to_error::deserialize(serde_ignored::Deserializer::new(
        value,
        &mut record_ignored,
    ))
    .map_err(|error| {
        let path = error_path(base, error.path());
        let reason = without_rust_type_names(&error.into_inner().to_string());
        if path.is_empty() {
            reason
        } else {
            format!("`{path}`: {reason}")
        }
    })?;

    if ignored.is_empty() {
        Ok(request)
    } else {
        Err(format!(
            "unknown field{} {}",
            if ignored.len() == 1 { "" } else { "s" },
            ignored
                .iter()
                .map(|path| format!("`{path}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

pub(crate) fn read_typed_config<T>(config_file: &str) -> CloudResult<T>
where
    T: DeserializeOwned,
{
    deserialize_strict_config(read_config_value(config_file)?, config_file)
}

/// Check the shared primary document argument contract for a concrete command.
#[cfg(test)]
pub(crate) fn assert_primary_json_input(args: &[&str], id: &str, aliases: &[&str]) {
    use crate::cli::Cli;
    use clap::{CommandFactory, Parser, error::ErrorKind};

    let base: Vec<_> = ["chctl"].into_iter().chain(args.iter().copied()).collect();
    assert_eq!(
        Cli::try_parse_from(&base).err().unwrap().kind(),
        ErrorKind::MissingRequiredArgument,
        "{args:?} requires a document or its existing alternative input"
    );
    let spellings: Vec<_> = ["file"]
        .into_iter()
        .chain(aliases.iter().copied())
        .collect();
    for spelling in &spellings {
        for input in ["private/request.json", "-"] {
            let flag = format!("--{spelling}");
            let mut argv = base.clone();
            argv.extend([flag.as_str(), input]);
            assert!(Cli::try_parse_from(&argv).is_ok(), "{argv:?}");
            let mut command = Cli::command();
            command.build();
            let matches = command.clone().try_get_matches_from(argv).unwrap();
            let mut leaf = &matches;
            let mut definition = &command;
            while let Some((name, child)) = leaf.subcommand() {
                leaf = child;
                definition = definition.find_subcommand(name).unwrap();
            }
            assert_eq!(leaf.get_raw(id).unwrap().collect::<Vec<_>>(), [input]);
            let arg = definition
                .get_arguments()
                .find(|arg| arg.get_id() == id)
                .unwrap();
            assert_eq!(arg.get_long(), Some("file"));
            assert_eq!(arg.get_value_names().unwrap(), ["PATH"]);
            assert!(!arg.is_hide_set());
            assert!(arg.get_visible_aliases().unwrap_or_default().is_empty());
            assert_eq!(arg.get_all_aliases().unwrap_or_default(), aliases);
        }
        for other in &spellings {
            let first = format!("--{spelling}");
            let second = format!("--{other}");
            let mut argv = base.clone();
            argv.extend([first.as_str(), "first.json", second.as_str(), "second.json"]);
            assert_eq!(
                Cli::try_parse_from(argv).err().unwrap().kind(),
                ErrorKind::ArgumentConflict,
                "{args:?}: {first} and {second} must not choose a file silently"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::client::CloudErrorKind;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        enabled: Option<bool>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Example {
        name: String,
        nested: Option<Nested>,
    }

    #[derive(Debug, Deserialize)]
    struct Empty {}

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    enum DistinctiveKind {
        Alpha,
        #[serde(untagged)]
        Other(String),
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct DistinctiveItem {
        labels: Vec<String>,
        kind: Option<DistinctiveKind>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct DistinctiveBody {
        name: String,
        items: Vec<DistinctiveItem>,
    }

    #[test]
    fn strict_errors_name_the_json_path_without_positions_or_rust_types() {
        let body = |items: Value| serde_json::json!({"name": "n", "items": items});
        for (value, path) in [
            (body(serde_json::json!(5)), "`items`"),
            (body(serde_json::json!([5])), "`items[0]`"),
            (
                body(serde_json::json!([{"labels": ["a"]}, {"labels": ["b", 7]}])),
                "`items[1].labels[1]`",
            ),
            (
                body(serde_json::json!([{"labels": [], "kind": 3}])),
                "`items[0].kind`",
            ),
            (serde_json::json!([]), ""),
            (serde_json::json!("body"), ""),
        ] {
            let error = deserialize_strict_config::<DistinctiveBody>(value, "test").unwrap_err();
            assert_eq!(error.kind, CloudErrorKind::Usage);
            let message = &error.message;
            assert!(message.contains(path), "{path} missing from {message}");
            // No Rust type name, and no position in a re-encoding of the body.
            for absent in ["Distinctive", "line", "column"] {
                assert!(!message.contains(absent), "{absent} in {message}");
            }
        }
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Numbers {
        ratio: f64,
        small_ratio: f32,
        offset: i64,
        tiny: i8,
        level: i32,
        count: u64,
        byte: u8,
        port: u16,
        size: u32,
        index: usize,
        positive: std::num::NonZeroU32,
    }

    #[test]
    fn strict_errors_name_json_numbers_instead_of_rust_primitives() {
        let valid = serde_json::json!({
            "ratio": 1.5, "small_ratio": 1.5, "offset": -1, "tiny": 1, "level": 1,
            "count": 1, "byte": 1, "port": 1, "size": 1, "index": 1, "positive": 1,
        });
        deserialize_strict_config::<Numbers>(valid.clone(), "test").unwrap();
        for (field, bad, rust_type) in [
            ("ratio", serde_json::json!("one"), "f64"),
            ("small_ratio", serde_json::json!(true), "f32"),
            ("offset", serde_json::json!(1.5), "i64"),
            ("tiny", serde_json::json!(200), "i8"),
            ("level", serde_json::json!("1"), "i32"),
            ("count", serde_json::json!(-1), "u64"),
            ("byte", serde_json::json!(300), "u8"),
            ("port", serde_json::json!(70000), "u16"),
            ("size", serde_json::json!(-1), "u32"),
            ("index", serde_json::json!("a"), "usize"),
            ("positive", serde_json::json!(0), "u32"),
            ("positive", serde_json::json!("a"), "u32"),
        ] {
            let mut value = valid.clone();
            value[field] = bad;
            let error = deserialize_strict_config::<Numbers>(value, "test").unwrap_err();
            assert_eq!(error.kind, CloudErrorKind::Usage);
            let message = &error.message;
            assert!(message.contains(&format!("`{field}`")), "{message}");
            let reason = message.rsplit_once("expected ").unwrap().1;
            assert!(!reason.contains(rust_type), "{rust_type} in {message}");
            assert!(!reason.contains("nonzero"), "{message}");
        }

        // A nested number reports its full path, not just the field name.
        let error =
            deserialize_strict_value::<Numbers>(serde_json::json!({"ratio": "one"}), "tiles[0]")
                .unwrap_err();
        assert!(error.contains("`tiles[0].ratio`"), "{error}");
        assert!(!error.contains("f64"), "{error}");
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Containers {
        labels: Vec<String>,
        settings: std::collections::BTreeMap<String, String>,
        tags: Option<Vec<String>>,
        name: String,
        nested: Nested,
    }

    #[test]
    fn strict_errors_name_json_containers_instead_of_serde_ones() {
        let valid = serde_json::json!({
            "labels": [], "settings": {}, "tags": null, "name": "n", "nested": {},
        });
        deserialize_strict_config::<Containers>(valid.clone(), "test").unwrap();
        for (field, bad) in [
            ("labels", serde_json::json!(5)),
            ("labels", serde_json::json!({})),
            ("settings", serde_json::json!([1])),
            ("settings", serde_json::json!("x")),
            ("tags", serde_json::json!({})),
            ("name", serde_json::json!([1])),
            ("name", serde_json::json!({})),
            ("nested", serde_json::json!(5)),
        ] {
            let mut value = valid.clone();
            value[field] = bad;
            let error = deserialize_strict_config::<Containers>(value, "test").unwrap_err();
            assert_eq!(error.kind, CloudErrorKind::Usage);
            let message = &error.message;
            assert!(message.contains(&format!("`{field}`")), "{message}");
            // serde's data-model words, on either side of the message.
            for absent in ["sequence", "map"] {
                assert!(!message.contains(absent), "{absent} in {message}");
            }
        }

        // An array too short for a struct is reported as a length, not a type.
        let error =
            deserialize_strict_config::<Containers>(serde_json::json!([]), "test").unwrap_err();
        assert!(!error.message.contains("sequence"), "{error}");
        assert!(!error.message.contains("Containers"), "{error}");
    }

    #[test]
    fn nested_strict_checks_report_paths_below_their_base() {
        let error = deserialize_strict_value::<DistinctiveItem>(
            serde_json::json!({"labels": [1]}),
            "tiles[0].config",
        )
        .unwrap_err();
        assert!(error.contains("`tiles[0].config.labels[0]`"), "{error}");

        let error = deserialize_strict_value::<DistinctiveItem>(
            serde_json::json!({"labels": [], "extra": true}),
            "tiles[0].config",
        )
        .unwrap_err();
        assert!(error.contains("`tiles[0].config.extra`"), "{error}");

        let error = deserialize_strict_config::<DistinctiveBody>(
            serde_json::json!({"name": "n", "items": [{"labels": [], "extra": true}]}),
            "test",
        )
        .unwrap_err();
        assert!(error.message.contains("`items[0].extra`"), "{error}");
    }

    #[test]
    fn strict_config_rejects_ignored_nested_fields_including_null() {
        let parsed: Example = deserialize_strict_config(
            serde_json::json!({"name": "ok", "nested": {"enabled": null}}),
            "test",
        )
        .unwrap();
        assert_eq!(
            parsed,
            Example {
                name: "ok".into(),
                nested: Some(Nested { enabled: None })
            }
        );

        let error = deserialize_strict_config::<Example>(
            serde_json::json!({"name": "bad", "nested": {"enabeld": null}}),
            "test",
        )
        .unwrap_err();
        assert!(error.message.contains("`nested.enabeld`"), "{error}");
        assert_eq!(error.kind, CloudErrorKind::Usage);
    }

    #[test]
    fn ignored_paths_preserve_map_keys_while_hiding_traversal_markers() {
        for key in ["?", "", "dotted.key"] {
            let error = deserialize_strict_config::<Empty>(serde_json::json!({key: true}), "test")
                .unwrap_err();
            assert!(error.message.contains(&format!("`{key}`")), "{error}");
        }
    }

    #[test]
    fn stdin_has_a_clear_config_source_label() {
        assert_eq!(config_source_label("-"), "stdin");
        assert_eq!(config_source_label("patch.json"), "patch.json");
    }

    #[test]
    fn read_config_value_names_unreadable_and_malformed_files() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.json");
        let missing = missing.to_str().unwrap();
        // Each failure names the file and carries the underlying error's text.
        let error = read_config_value(missing).unwrap_err();
        assert!(error.message.contains("--file"), "{error}");
        let io_error = std::fs::read_to_string(missing).unwrap_err().to_string();
        assert!(
            error.message.contains(missing) && error.message.ends_with(&io_error),
            "{error}"
        );
        // A file that cannot be read is a runtime failure, not a usage error.
        assert_eq!(error.kind, CloudErrorKind::Generic);
        assert_eq!(
            error.failure.map(|failure| failure.kind),
            Some(FailureKind::Io)
        );

        let malformed = directory.path().join("malformed.json");
        std::fs::write(&malformed, r#"{"entries":["#).unwrap();
        let malformed = malformed.to_str().unwrap();
        let error = read_config_value(malformed).unwrap_err();
        let serde_error = serde_json::from_str::<serde_json::Value>(r#"{"entries":["#)
            .unwrap_err()
            .to_string();
        assert!(
            error.message.contains(malformed) && error.message.ends_with(&serde_error),
            "{error}"
        );
        assert_eq!(error.kind, CloudErrorKind::Usage);

        let valid = directory.path().join("valid.json");
        std::fs::write(&valid, r#"{"name":"ok"}"#).unwrap();
        assert_eq!(
            read_config_value(valid.to_str().unwrap()).unwrap(),
            serde_json::json!({"name": "ok"})
        );
    }
}
