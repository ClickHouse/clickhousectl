//! `local udf`: executable UDFs for project-local ClickHouse servers.
//!
//! A UDF lives in a directory (`clickhouse/udfs/<name>/`) holding
//! `udf.json`, the same definition `cloud udf` accepts, next to its files.
//! `deploy` renders that definition into ClickHouse's `<function>` XML under
//! the server's data directory, copies the sources next to it, and points the
//! server at both through a managed `config.d` overlay. The server picks new
//! files up on its own; when it is running we also ask it to reload at once.

use crate::error::{Error, Result, UdfCommandPath, UdfRejection};
use crate::local::cli::{UdfCommands, UdfRuntimeArg, UdfTypeArg};
use crate::local::output::{
    self, UdfDeployOutput, UdfInitOutput, UdfListEntry, UdfListOutput, UdfReloadOutput,
    UdfRemoveOutput,
};
use crate::local::server::{self, Engine, MetadataLock, ServerInfo};
use crate::udf::{
    self, DEFAULT_UDF_PARENT, DEFINITION_FILE, NATIVE_ARCH_DIRS, NATIVE_ENTRYPOINT,
    PYTHON_ENTRYPOINT, SourceEntry, UdfInputError, UdfRuntimeKind,
};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Managed `config.d` overlay that points the server at the directories below.
pub(crate) const OVERLAY_FILE: &str = "chctl-udf.xml";
/// Rendered `<function>` files, matched by the overlay's glob.
const FUNCTIONS_DIR: &str = "user_defined_functions";
/// Copied sources; ClickHouse resolves direct commands inside this directory.
const SCRIPTS_DIR: &str = "user_scripts";
const FUNCTION_FILE_SUFFIX: &str = "_function.xml";
/// The longest file name a local UDF name is embedded in is `stage_scripts`'s
/// `.<name>.staging-<pid>` (a `u32` pid has at most 10 digits).
const LONGEST_NAME_OVERHEAD: usize = ".".len() + ".staging-".len() + 10;
/// Neither ClickHouse nor the Cloud API documents a function name limit, but
/// every local file named after the function must fit in a 255-byte file
/// name (`NAME_MAX` on Linux and macOS).
pub(crate) const MAX_LOCAL_NAME_LEN: usize = 255 - LONGEST_NAME_OVERHEAD;
/// Present next to a function's files while the running server rejected
/// their last deploy, so whatever is loaded is an earlier definition.
const REJECTED_MARKER_SUFFIX: &str = ".rejected";
const DEFAULT_FORMAT: &str = "TabSeparated";
const PYTHON_CANDIDATES: [&str; 2] = ["python3.11", "python3"];
/// The Python version Cloud runs `python3.11` UDFs with.
const CLOUD_PYTHON_VERSION: &str = "3.11";
/// How long the interpreter may take to report its version.
const PYTHON_VERSION_TIMEOUT: Duration = Duration::from_secs(5);
/// Copied but not installed: a local server runs the interpreter as is.
const REQUIREMENTS_FILE: &str = "requirements.txt";
const REQUIREMENTS_NOTICE: &str = "requirements.txt is not installed locally; install its packages \
     into the interpreter you deploy with (e.g. a virtualenv passed with --python).";
const NATIVE_UNSUPPORTED: &str = "native UDFs run only on local servers on Linux amd64 or \
     arm64; deploy from a Linux host, or to Cloud with `cloud udf deploy`";
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// How long `deploy` keeps asking a running server to reload before giving
/// up on the function. Covers the server's own config-reload period.
const LOAD_POLL_TIMEOUT: Duration = Duration::from_secs(10);
const LOAD_POLL_INTERVAL: Duration = Duration::from_millis(250);
const RELOAD_FUNCTIONS_SQL: &str = "SYSTEM RELOAD FUNCTIONS";
const LOADED_FUNCTIONS_SQL: &str = "SELECT name FROM system.functions \
     WHERE origin = 'ExecutableUserDefined' ORDER BY name FORMAT TabSeparated";
/// Per-function load status, ClickHouse 26.2 and later. A function that
/// loaded before keeps `load_status = 'Success'` (it still runs that earlier
/// definition) when its changed file is rejected, but the rejection is in
/// `loading_error_message`. Older servers fail this query.
const FAILED_FUNCTIONS_SQL: &str = "SELECT name FROM system.user_defined_functions \
     WHERE load_status = 'Failed' OR loading_error_message != '' ORDER BY name \
     FORMAT TabSeparated";

/// A `local udf` NAME: the shared function-name rule, and short enough for
/// every file the CLI names after it. Names are ASCII, so bytes are chars.
pub(crate) fn validate_local_function_name(name: &str) -> std::result::Result<(), String> {
    udf::validate_function_name(name)?;
    if name.len() > MAX_LOCAL_NAME_LEN {
        return Err(format!(
            "Use at most {MAX_LOCAL_NAME_LEN} characters; this name has {}",
            name.len()
        ));
    }
    Ok(())
}

pub async fn run(cmd: UdfCommands, json: bool) -> Result<()> {
    match cmd {
        UdfCommands::Init {
            name,
            runtime,
            kind,
        } => init_udf(&name, runtime, kind, json),
        UdfCommands::Deploy {
            name,
            dir,
            server,
            python,
        } => deploy(&name, &dir.dir, &server.server, python.as_deref(), json).await,
        UdfCommands::List { server } => list(&server.server, json).await,
        UdfCommands::Remove { name, server } => remove(&name, &server.server, json).await,
        UdfCommands::Reload { server } => reload(&server.server, json).await,
    }
}

// ── definition ──────────────────────────────────────────────────────────────

/// The `udf.json` shape, typed. Field names are the Cloud API's; unknown
/// fields are rejected so a typo never silently falls back to a default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalUdfDefinition {
    pub function_name: String,
    #[serde(rename = "type")]
    pub kind: LocalUdfType,
    pub runtime: LocalUdfRuntime,
    pub arguments: Vec<LocalUdfArgument>,
    pub return_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_read_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_write_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_command_execution_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_chunk_header: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deterministic: Option<bool>,
    /// Cloud-only; accepted and reported as ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mib: Option<u64>,
    /// Cloud-only; accepted and reported as ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_type: Option<String>,
    /// Cloud-only; accepted and reported as ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_version: Option<String>,
}

impl LocalUdfDefinition {
    /// Fields present in the definition that the local XML leaves out, in
    /// the API's spelling: the Cloud-only ones, and the pool settings on a
    /// plain `executable`, which ClickHouse reads only for `executable_pool`.
    fn ignored_fields(&self) -> Vec<String> {
        let executable = self.kind == LocalUdfType::Executable;
        [
            ("poolSize", executable && self.pool_size.is_some()),
            (
                "maxCommandExecutionTime",
                executable && self.max_command_execution_time.is_some(),
            ),
            ("memoryLimitMib", self.memory_limit_mib.is_some()),
            ("sandboxType", self.sandbox_type.is_some()),
            ("sandboxVersion", self.sandbox_version.is_some()),
        ]
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(name, _)| name.to_owned())
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum LocalUdfType {
    #[serde(rename = "executable")]
    Executable,
    #[serde(rename = "executable_pool")]
    ExecutablePool,
}

impl LocalUdfType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::ExecutablePool => "executable_pool",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum LocalUdfRuntime {
    #[serde(rename = "python3.11")]
    Python311,
    #[serde(rename = "native")]
    Native,
}

impl LocalUdfRuntime {
    fn kind(self) -> UdfRuntimeKind {
        match self {
            Self::Python311 => UdfRuntimeKind::Python311,
            Self::Native => UdfRuntimeKind::Native,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocalUdfArgument {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Load and validate `dir/udf.json` the same way `cloud udf create` does,
/// check that it names `name`, then type it. Returns the definition and the
/// fields the local XML leaves out.
fn load_local_definition(dir: &Path, name: &str) -> Result<(LocalUdfDefinition, Vec<String>)> {
    let value = udf::load_definition_from_dir(dir).map_err(input_error)?;
    let path = dir.join(DEFINITION_FILE);
    let invalid = |reason: String| Error::UdfDefinitionInvalid {
        path: path.clone(),
        reason,
    };
    udf::validate_definition(&value, true).map_err(invalid)?;
    udf::check_function_name(&value, name).map_err(invalid)?;
    let definition: LocalUdfDefinition =
        serde_json::from_value(value).map_err(|error| invalid(error.to_string()))?;
    let ignored = definition.ignored_fields();
    Ok((definition, ignored))
}

fn input_error(error: UdfInputError) -> Error {
    match error {
        UdfInputError::Read { path, source }
            if source.kind() == std::io::ErrorKind::NotFound
                && path.file_name() == Some(OsStr::new(DEFINITION_FILE)) =>
        {
            Error::UdfSourceInvalid {
                path: path.parent().map(Path::to_path_buf).unwrap_or(path),
                reason: format!("has no {DEFINITION_FILE}"),
            }
        }
        UdfInputError::Read { source, .. } => Error::Io(source),
        UdfInputError::Parse { path, source } => Error::UdfDefinitionParse { path, source },
        UdfInputError::Invalid { path, reason } => Error::UdfSourceInvalid { path, reason },
        UdfInputError::Missing { path } => Error::UdfSourceInvalid {
            path,
            reason: udf::SOURCE_DIR_MISSING.to_owned(),
        },
    }
}

// ── init ────────────────────────────────────────────────────────────────────

fn init_udf(name: &str, runtime: UdfRuntimeArg, kind: UdfTypeArg, json: bool) -> Result<()> {
    let parent = Path::new(DEFAULT_UDF_PARENT);
    if let Some(existing) = case_only_clash(parent, name)? {
        return Err(Error::Usage(Box::new(clap::Error::raw(
            clap::error::ErrorKind::ValueValidation,
            format!(
                "UDF {existing} already exists in {DEFAULT_UDF_PARENT} and differs from {name} \
                 only by case; use {existing} or choose another name\n"
            ),
        ))));
    }
    let target = parent.join(name);
    std::fs::create_dir_all(&target)?;

    let mut created = Vec::new();
    write_if_absent(
        &target,
        DEFINITION_FILE,
        &definition_template(name, runtime, kind),
        0o644,
        &mut created,
    )?;
    let next_step = match runtime {
        UdfRuntimeArg::Python311 => {
            write_if_absent(
                &target,
                PYTHON_ENTRYPOINT,
                &python_template(name),
                0o755,
                &mut created,
            )?;
            None
        }
        UdfRuntimeArg::Native => {
            for arch in NATIVE_ARCH_DIRS {
                let arch_dir = target.join(arch);
                if !arch_dir.is_dir() {
                    std::fs::create_dir_all(&arch_dir)?;
                    created.push(format!("{arch}/"));
                }
                // `.gitkeep` only keeps the empty directory in version control;
                // the directory is what gets reported.
                write_if_absent(
                    &target,
                    &format!("{arch}/.gitkeep"),
                    "",
                    0o644,
                    &mut Vec::new(),
                )?;
            }
            Some(native_next_step())
        }
    };
    let warnings: Vec<String> = match runtime {
        UdfRuntimeArg::Native => native_host_arch()
            .err()
            .map(|error| error.to_string())
            .into_iter()
            .collect(),
        UdfRuntimeArg::Python311 => Vec::new(),
    };
    if !json {
        for warning in &warnings {
            // Not `eprintln!`, which panics on a closed stderr.
            let _ = writeln!(std::io::stderr(), "Warning: {warning}");
        }
    }

    let out = UdfInitOutput {
        name: name.to_owned(),
        dir: target.display().to_string(),
        created,
        next_step,
        warnings,
    };
    output::print_output(&out, json);
    Ok(())
}

/// An existing UDF directory under `parent` whose name equals `name` except
/// for case. Case-insensitive filesystems (the macOS default) would resolve
/// `name` to that directory, whose `functionName` then no longer matches.
fn case_only_clash(parent: &Path, name: &str) -> Result<Option<String>> {
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let existing = entry?.file_name().to_string_lossy().into_owned();
        if existing != name && existing.eq_ignore_ascii_case(name) {
            return Ok(Some(existing));
        }
    }
    Ok(None)
}

/// What a `native` scaffold still needs before it can run.
fn native_next_step() -> String {
    format!(
        "Build linux/amd64 and linux/arm64 binaries into amd64/{NATIVE_ENTRYPOINT} and \
         arm64/{NATIVE_ENTRYPOINT} (see {NATIVE_DOCS_URL})."
    )
}

const NATIVE_DOCS_URL: &str = "https://clickhouse.com/docs/products/cloud/features/sql-console-features/user-defined-functions";

/// Create `dir/relative` with `contents` unless it already exists, recording
/// `relative` in `created`. Existing files are kept untouched so re-running
/// `init` never discards edits.
fn write_if_absent(
    dir: &Path,
    relative: &str,
    contents: &str,
    mode: u32,
    created: &mut Vec<String>,
) -> Result<()> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(relative))
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            created.push(relative.to_owned());
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// The scaffolded `udf.json`: a one-argument String function in the Cloud
/// API's field names, valid for both `local udf deploy` and `cloud udf create`.
pub(crate) fn definition_template(name: &str, runtime: UdfRuntimeArg, kind: UdfTypeArg) -> String {
    let value = serde_json::json!({
        "functionName": name,
        "type": kind.as_str(),
        "runtime": runtime.as_str(),
        "arguments": [{"name": "value", "type": "String"}],
        "returnType": "String",
        "format": DEFAULT_FORMAT,
        "deterministic": false,
    });
    let mut text = serde_json::to_string_pretty(&value).expect("static template serializes");
    text.push('\n');
    text
}

fn python_template(name: &str) -> String {
    format!(
        "#!/usr/bin/env python3\n\
         \"\"\"{name}: executable UDF entrypoint.\n\
         \n\
         ClickHouse writes one TabSeparated row per line to stdin and reads one result\n\
         line per row from stdout. Flush after every row so pooled processes never stall.\n\
         \"\"\"\n\
         import sys\n\
         \n\
         \n\
         def transform(value: str) -> str:\n\
         \x20   return value\n\
         \n\
         \n\
         def main() -> None:\n\
         \x20   # Values arrive escaped (\\t, \\n, \\\\): unescape before transforming, then escape the result.\n\
         \x20   for line in sys.stdin:\n\
         \x20       print(transform(line.rstrip(\"\\n\")))\n\
         \x20       sys.stdout.flush()\n\
         \n\
         \n\
         if __name__ == \"__main__\":\n\
         \x20   main()\n"
    )
}

// ── deploy ──────────────────────────────────────────────────────────────────

async fn deploy(
    name: &str,
    parent: &Path,
    server_name: &str,
    python: Option<&Path>,
    json: bool,
) -> Result<()> {
    // Everything that can be wrong with the input fails here, before the
    // server is even looked up and long before anything is written.
    let dir = udf::resolve_source_dir(parent, name).map_err(|error| match error {
        UdfInputError::Missing { path } => Error::UdfSourceMissing {
            path,
            name: name.to_owned(),
            scaffold_here: parent == Path::new(DEFAULT_UDF_PARENT),
        },
        other => input_error(other),
    })?;
    let (definition, ignored_fields) = load_local_definition(&dir, name)?;
    let runtime = definition.runtime.kind();
    if runtime == UdfRuntimeKind::Native && python.is_some() {
        return Err(Error::Usage(Box::new(clap::Error::raw(
            clap::error::ErrorKind::ArgumentConflict,
            format!(
                "--python applies only to runtime python3.11, but UDF {name} uses runtime native\n"
            ),
        ))));
    }
    let native_arch = match runtime {
        UdfRuntimeKind::Native => Some(native_host_arch()?),
        UdfRuntimeKind::Python311 => None,
    };
    // A local server runs only its host's binary, so only that one is needed.
    let entries = match native_arch {
        Some(arch) => udf::collect_source_entries_for(&dir, runtime, &[arch]),
        None => udf::collect_source_entries(&dir, runtime),
    }
    .map_err(input_error)?;
    let ignored_files = ignored_files(&dir, &entries, runtime, native_arch);

    let (target, mut lock) = server_target(server_name)?;
    let data_dir_abs = target.data_dir.canonicalize()?;
    let (command, interpreter) = match native_arch {
        Some(arch) => (
            ResolvedCommand {
                command: format!("{name}/{arch}/{NATIVE_ENTRYPOINT}"),
                execute_direct: true,
            },
            None,
        ),
        None => python_command(name, &data_dir_abs, python)?,
    };
    let mut warnings = Vec::new();
    if let Some(interpreter) = &interpreter {
        let version = python_version(interpreter).await;
        warnings.extend(python_version_warning(interpreter, version.as_deref()));
    }
    // A name ClickHouse already knows makes every later reload fail (or is
    // never callable), so it is refused before anything is written. Only a
    // running server can say; the lock is not held across the round trip.
    let mut was_loaded = false;
    if let Some(info) = &target.running {
        drop(lock);
        was_loaded = check_name_is_free(info, server_name, name).await?;
        lock = server::lock_metadata()?;
    }

    let scripts_dir = target.data_dir.join(SCRIPTS_DIR).join(name);
    let executables: Vec<PathBuf> = native_arch
        .map(|arch| Path::new(arch).join(NATIVE_ENTRYPOINT))
        .into_iter()
        .collect();
    stage_scripts(&entries, &scripts_dir, &executables)?;
    std::fs::create_dir_all(target.data_dir.join(FUNCTIONS_DIR))?;
    let function_config = function_xml_path(&target.data_dir, name);
    let previous_mtime = modified_at(&function_config);
    write_atomic(
        &function_config,
        &render_function_xml(&definition, &command),
    )?;
    bump_mtime_past(&function_config, previous_mtime)?;
    let mut sidecar = serde_json::to_string_pretty(&definition)?;
    sidecar.push('\n');
    write_atomic(&sidecar_path(&target.data_dir, name), &sidecar)?;
    // These files have not been rejected yet; a rejection below marks them.
    remove_if_present(&rejected_marker_path(&target.data_dir, name))?;
    // A rewritten overlay takes effect only once the server rereads its
    // config, so until then the server may not see any function file.
    let overlay_fresh = !overlay_is_current(&target.data_dir)?;
    write_overlay(&target.data_dir)?;
    drop(lock);

    let loaded = match &target.running {
        Some(info) => {
            let deploy_command = deploy_command(name, parent, server_name);
            let probe = LoadProbe {
                name,
                data_dir: &target.data_dir,
                was_loaded,
                overlay_fresh,
            };
            match reload_until_loaded(info, server_name, &probe).await? {
                LoadOutcome::Loaded(loaded) => clear_rejected_markers(&target.data_dir, &loaded)?,
                LoadOutcome::LoadedDespite {
                    current,
                    broken,
                    details,
                } => {
                    clear_rejected_markers(&target.data_dir, &current)?;
                    warnings.push(reload_still_fails_warning(server_name, &broken, &details));
                }
                LoadOutcome::NotLoaded => {
                    return Err(Error::UdfNotLoaded {
                        name: name.to_owned(),
                        server: server_name.to_owned(),
                        log_path: std::path::absolute(server::server_log_path(server_name))?,
                        deploy_command,
                    });
                }
                LoadOutcome::Rejected { details, blocking } => {
                    // Best effort: failing to mark must not hide the rejection.
                    let _ = std::fs::write(rejected_marker_path(&target.data_dir, name), "");
                    return Err(Error::UdfRejected(Box::new(UdfRejection {
                        name: name.to_owned(),
                        server: server_name.to_owned(),
                        source_dir: dir.clone(),
                        deploy_command,
                        blocking,
                        details,
                    })));
                }
            }
            Some(true)
        }
        None => None,
    };

    if !json {
        // Not `eprintln!`, which panics on a closed stderr.
        for warning in &warnings {
            let _ = writeln!(std::io::stderr(), "Warning: {warning}");
        }
        if ignored_files.iter().any(|file| file == REQUIREMENTS_FILE) {
            let _ = writeln!(std::io::stderr(), "{REQUIREMENTS_NOTICE}");
        }
    }
    let out = UdfDeployOutput {
        name: name.to_owned(),
        server: server_name.to_owned(),
        r#type: definition.kind.as_str().to_owned(),
        runtime: runtime.as_str().to_owned(),
        reloaded: target.running.is_some(),
        loaded,
        interpreter: interpreter.map(|path| path.display().to_string()),
        ignored_fields,
        ignored_files,
        warnings,
        function_config: display_path(&function_config),
        scripts_dir: display_path(&scripts_dir),
    };
    output::print_output(&out, json);
    Ok(())
}

/// The command that reruns this deployment, naming `--dir` only when the user
/// gave a non-default one, spelled as they gave it.
fn deploy_command(name: &str, parent: &Path, server: &str) -> String {
    if parent == Path::new(udf::DEFAULT_UDF_PARENT) {
        format!("clickhousectl local udf deploy {name} --server {server}")
    } else {
        format!(
            "clickhousectl local udf deploy {name} --dir {} --server {server}",
            parent.display()
        )
    }
}

/// Source files the local server does nothing with: `requirements.txt`
/// (copied, not installed) and, for `native`, the other architecture's
/// directory (not copied; Cloud uses it). Directories end in `/`.
fn ignored_files(
    dir: &Path,
    entries: &[SourceEntry],
    runtime: UdfRuntimeKind,
    native_arch: Option<&str>,
) -> Vec<String> {
    match (runtime, native_arch) {
        (UdfRuntimeKind::Python311, _) => entries
            .iter()
            .any(|entry| !entry.is_dir && entry.relative == Path::new(REQUIREMENTS_FILE))
            .then(|| REQUIREMENTS_FILE.to_owned())
            .into_iter()
            .collect(),
        (UdfRuntimeKind::Native, host) => NATIVE_ARCH_DIRS
            .into_iter()
            .filter(|arch| Some(*arch) != host && dir.join(arch).is_dir())
            .map(|arch| format!("{arch}/"))
            .collect(),
    }
}

/// The architecture directory a native UDF runs from on this host. Native
/// UDFs are Linux binaries, so only a Linux amd64/arm64 host qualifies.
fn native_host_arch() -> Result<&'static str> {
    native_arch_for(std::env::consts::OS, std::env::consts::ARCH)
}

fn native_arch_for(os: &str, arch: &str) -> Result<&'static str> {
    if os != "linux" {
        return Err(Error::UdfRuntimeUnsupported(NATIVE_UNSUPPORTED.into()));
    }
    match arch {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        other => Err(Error::UdfRuntimeUnsupported(format!(
            "native UDFs run only on local servers on Linux amd64 or arm64, but this host's \
             CPU is {other}; deploy from an amd64 or arm64 host, or to Cloud with \
             `cloud udf deploy`"
        ))),
    }
}

/// The `<command>` and `<execute_direct>` pair for a definition.
///
/// `python3.11` runs through `sh -c` with an absolute interpreter and script
/// path, so no shebang or execute bit is needed, matching Cloud. `native`
/// runs directly: ClickHouse resolves the first token inside
/// `user_scripts_path`, where the sources were copied.
struct ResolvedCommand {
    command: String,
    execute_direct: bool,
}

fn python_command(
    name: &str,
    data_dir_abs: &Path,
    python: Option<&Path>,
) -> Result<(ResolvedCommand, Option<PathBuf>)> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let interpreter = resolve_python(python, &path_var)?;
    let script = data_dir_abs
        .join(SCRIPTS_DIR)
        .join(name)
        .join(PYTHON_ENTRYPOINT);
    // `exec` replaces the shell, so each pooled worker is one python process.
    let command = format!(
        "exec {} {}",
        shell_quote(&interpreter, UdfCommandPath::Interpreter)?,
        shell_quote(&script, UdfCommandPath::StagedScript)?
    );
    Ok((
        ResolvedCommand {
            command,
            execute_direct: false,
        },
        Some(interpreter),
    ))
}

/// `--python` wins; otherwise the first of `python3.11`, `python3` on PATH.
/// A bare `--python NAME` is also looked up on PATH.
fn resolve_python(explicit: Option<&Path>, path_var: &OsStr) -> Result<PathBuf> {
    if let Some(path) = explicit {
        let found = if path.components().count() == 1 {
            find_in_path(path.as_os_str(), path_var)
        } else {
            is_executable_file(path).then(|| path.to_path_buf())
        };
        return found.map(absolute).transpose()?.ok_or_else(|| {
            Error::UdfInterpreterNotFound(format!(
                "Python interpreter '{}' is not an executable file",
                path.display()
            ))
        });
    }
    PYTHON_CANDIDATES
        .iter()
        .find_map(|name| find_in_path(OsStr::new(name), path_var))
        .map(absolute)
        .transpose()?
        .ok_or_else(|| {
            Error::UdfInterpreterNotFound(
                "No python3.11 or python3 interpreter found on PATH; pass --python <PATH>".into(),
            )
        })
}

fn find_in_path(name: &OsStr, path_var: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path_var)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Absolute and lexically normalised (`.` and `..` folded away), so the
/// command carries `/home/me/py3`, never `proj/../py3`. Symbolic links are
/// kept: a virtualenv's `bin/python` must stay the link it is.
fn absolute(path: PathBuf) -> Result<PathBuf> {
    let mut normalised = PathBuf::new();
    for component in std::path::absolute(path)?.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalised.pop();
            }
            other => normalised.push(other),
        }
    }
    Ok(normalised)
}

/// The `major.minor` version an interpreter reports, or `None` when it
/// cannot be run, fails, prints something else or takes too long.
async fn python_version(interpreter: &Path) -> Option<String> {
    let mut command = tokio::process::Command::new(interpreter);
    command
        .args(["-c", "import sys; print('%d.%d' % sys.version_info[:2])"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(PYTHON_VERSION_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let version = text.trim();
    let (major, minor) = version.split_once('.')?;
    let numeric = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    (numeric(major) && numeric(minor)).then(|| version.to_owned())
}

/// Cloud runs python3.11; say so when the local interpreter differs or its
/// version could not be determined.
fn python_version_warning(interpreter: &Path, version: Option<&str>) -> Option<String> {
    match version {
        Some(CLOUD_PYTHON_VERSION) => None,
        Some(version) => Some(format!(
            "interpreter {} is Python {version}, but Cloud runs Python {CLOUD_PYTHON_VERSION}; \
             pass --python with a 3.11 interpreter to match",
            interpreter.display()
        )),
        None => Some(format!(
            "could not determine the Python version of interpreter {}; Cloud runs Python \
             {CLOUD_PYTHON_VERSION}",
            interpreter.display()
        )),
    }
}

/// Single-quote a path for `sh -c`, which ClickHouse uses when
/// `execute_direct` is off. Single quotes inside the path cannot be carried.
fn shell_quote(path: &Path, role: UdfCommandPath) -> Result<String> {
    let text = path
        .to_str()
        .filter(|text| !text.contains('\''))
        .ok_or_else(|| Error::UdfPathUnquotable {
            role,
            path: path.to_path_buf(),
        })?;
    Ok(format!("'{text}'"))
}

/// Copy the sources into `target`, replacing any previous copy atomically:
/// build a sibling staging directory, then swap it in. `executables` are
/// made runnable after the copy.
fn stage_scripts(entries: &[SourceEntry], target: &Path, executables: &[PathBuf]) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| std::io::Error::other("scripts directory has no parent"))?;
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    std::fs::create_dir_all(parent)?;
    let pid = std::process::id();
    let staging = parent.join(format!(".{name}.staging-{pid}"));
    let retired = parent.join(format!(".{name}.retired-{pid}"));
    for stale in [&staging, &retired] {
        if stale.exists() {
            std::fs::remove_dir_all(stale)?;
        }
    }
    std::fs::create_dir_all(&staging)?;
    for entry in entries {
        let destination = staging.join(&entry.relative);
        if entry.is_dir {
            std::fs::create_dir_all(&destination)?;
        } else {
            std::fs::copy(&entry.absolute, &destination)?;
            std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(entry.mode))?;
        }
    }
    for file in executables {
        std::fs::set_permissions(staging.join(file), std::fs::Permissions::from_mode(0o755))?;
    }
    if target.exists() {
        std::fs::rename(target, &retired)?;
    }
    std::fs::rename(&staging, target)?;
    if retired.exists() {
        std::fs::remove_dir_all(&retired)?;
    }
    Ok(())
}

// ── XML ─────────────────────────────────────────────────────────────────────

fn render_function_xml(definition: &LocalUdfDefinition, command: &ResolvedCommand) -> String {
    let mut xml = String::from("<clickhouse>\n    <function>\n");
    let mut element = |name: &str, value: &str| {
        xml.push_str(&format!("        <{name}>{}</{name}>\n", xml_escape(value)));
    };
    element("type", definition.kind.as_str());
    element("name", &definition.function_name);
    element("command", &command.command);
    element(
        "execute_direct",
        if command.execute_direct { "1" } else { "0" },
    );
    element(
        "format",
        definition.format.as_deref().unwrap_or(DEFAULT_FORMAT),
    );
    for argument in &definition.arguments {
        xml.push_str("        <argument>\n");
        xml.push_str(&format!(
            "            <type>{}</type>\n",
            xml_escape(&argument.kind)
        ));
        xml.push_str(&format!(
            "            <name>{}</name>\n",
            xml_escape(&argument.name)
        ));
        xml.push_str("        </argument>\n");
    }
    let mut element = |name: &str, value: &str| {
        xml.push_str(&format!("        <{name}>{}</{name}>\n", xml_escape(value)));
    };
    element("return_type", &definition.return_type);
    if let Some(name) = &definition.return_name {
        element("return_name", name);
    }
    if let Some(value) = definition.command_read_timeout {
        element("command_read_timeout", &value.to_string());
    }
    if let Some(value) = definition.command_write_timeout {
        element("command_write_timeout", &value.to_string());
    }
    if definition.kind == LocalUdfType::ExecutablePool {
        if let Some(value) = definition.pool_size {
            element("pool_size", &value.to_string());
        }
        if let Some(value) = definition.max_command_execution_time {
            element("max_command_execution_time", &value.to_string());
        }
    }
    if let Some(value) = definition.send_chunk_header {
        element("send_chunk_header", if value { "true" } else { "false" });
    }
    if let Some(value) = definition.deterministic {
        element("deterministic", if value { "true" } else { "false" });
    }
    xml.push_str("    </function>\n</clickhouse>\n");
    xml
}

fn render_overlay_xml(data_dir_abs: &Path) -> String {
    let glob = format!(
        "{}/*{FUNCTION_FILE_SUFFIX}",
        data_dir_abs.join(FUNCTIONS_DIR).display()
    );
    let scripts = format!("{}/", data_dir_abs.join(SCRIPTS_DIR).display());
    format!(
        "<clickhouse>\n    \
         <!-- Managed by clickhousectl: rewritten by server start and udf deploy/remove when its content changes. -->\n    \
         <user_defined_executable_functions_config>{}</user_defined_executable_functions_config>\n    \
         <user_scripts_path>{}</user_scripts_path>\n\
         </clickhouse>\n",
        xml_escape(&glob),
        xml_escape(&scripts)
    )
}

fn xml_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            other => escaped.push(other),
        }
    }
    escaped
}

// ── layout ──────────────────────────────────────────────────────────────────

/// Write the managed overlay that points the embedded server config at this
/// server's UDF directories. Idempotent; called by `server start` as well.
///
/// An unchanged overlay is left untouched: a new mtime makes ClickHouse
/// reload its main config, which on 26.1 and earlier drops the command-line
/// port overrides and rebinds the server to the default ports (#1066).
pub(crate) fn write_overlay(data_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(data_dir)?;
    let data_dir_abs = data_dir.canonicalize()?;
    let config_d = data_dir.join("config.d");
    std::fs::create_dir_all(&config_d)?;
    let path = config_d.join(OVERLAY_FILE);
    let rendered = render_overlay_xml(&data_dir_abs);
    if std::fs::read(&path).ok().as_deref() != Some(rendered.as_bytes()) {
        write_atomic(&path, &rendered)?;
    }
    Ok(path)
}

/// Whether the overlay [`write_overlay`] would write is already in place.
fn overlay_is_current(data_dir: &Path) -> Result<bool> {
    let path = data_dir.join("config.d").join(OVERLAY_FILE);
    let rendered = render_overlay_xml(&data_dir.canonicalize()?);
    Ok(std::fs::read(path).ok().as_deref() == Some(rendered.as_bytes()))
}

fn function_xml_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir
        .join(FUNCTIONS_DIR)
        .join(format!("{name}{FUNCTION_FILE_SUFFIX}"))
}

fn sidecar_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(FUNCTIONS_DIR).join(format!("{name}.json"))
}

fn rejected_marker_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir
        .join(FUNCTIONS_DIR)
        .join(format!("{name}{REJECTED_MARKER_SUFFIX}"))
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// After a reload succeeded, every loaded function runs its current files:
/// ClickHouse fails the whole reload when a changed file is rejected.
fn clear_rejected_markers(data_dir: &Path, loaded: &[String]) -> Result<()> {
    for name in loaded {
        remove_if_present(&rejected_marker_path(data_dir, name))?;
    }
    Ok(())
}

/// Whether the running server rejected the last deploy of `name`'s files.
/// A server started after that deploy loaded the current files, so a
/// function it has loaded is no longer stale.
fn last_deploy_rejected(
    data_dir: &Path,
    name: &str,
    running: Option<&ServerInfo>,
    loaded: Option<bool>,
) -> bool {
    let Some(marked) = modified_at(&rejected_marker_path(data_dir, name)) else {
        return false;
    };
    let started_after = running
        .and_then(|info| info.started_at.parse::<u64>().ok())
        .zip(marked.duration_since(SystemTime::UNIX_EPOCH).ok())
        .is_some_and(|(started, marked)| started > marked.as_secs());
    !(loaded == Some(true) && started_after)
}

fn modified_at(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// ClickHouse notices a changed function file by its modification time at
/// one-second resolution, so a redeploy within the same second as the
/// previous one would be served from the old definition and still look
/// loaded. Push the new file's mtime past the previous one.
fn bump_mtime_past(path: &Path, previous: Option<SystemTime>) -> Result<()> {
    let Some(previous) = previous else {
        return Ok(());
    };
    let seconds = |time: SystemTime| {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0)
    };
    let Some(current) = modified_at(path) else {
        return Ok(());
    };
    if seconds(current) <= seconds(previous) {
        std::fs::File::options()
            .write(true)
            .open(path)?
            .set_modified(previous + Duration::from_secs(1))?;
    }
    Ok(())
}

/// Write via a dot-prefixed temporary name that neither ClickHouse's
/// `config.d` merge nor the function glob matches, then rename into place.
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));
    std::fs::write(&temporary, contents)?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

struct StagedUdf {
    name: String,
    kind: Option<String>,
    runtime: Option<String>,
}

/// Every rendered function file, with type and runtime from its sidecar.
fn staged_udfs(data_dir: &Path) -> Result<Vec<StagedUdf>> {
    let functions_dir = data_dir.join(FUNCTIONS_DIR);
    let mut staged = Vec::new();
    let entries = match std::fs::read_dir(&functions_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(staged),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(name) = file_name.strip_suffix(FUNCTION_FILE_SUFFIX) else {
            continue;
        };
        if name.is_empty() || file_name.starts_with('.') {
            continue;
        }
        let definition = std::fs::read_to_string(sidecar_path(data_dir, name))
            .ok()
            .and_then(|text| serde_json::from_str::<LocalUdfDefinition>(&text).ok());
        staged.push(StagedUdf {
            name: name.to_owned(),
            kind: definition
                .as_ref()
                .map(|definition| definition.kind.as_str().to_owned()),
            runtime: definition
                .as_ref()
                .map(|definition| definition.runtime.kind().as_str().to_owned()),
        });
    }
    staged.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(staged)
}

fn display_path(path: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(&cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
}

// ── server ──────────────────────────────────────────────────────────────────

struct ServerTarget {
    data_dir: PathBuf,
    /// Metadata of the running server, when it is running.
    running: Option<ServerInfo>,
}

/// Resolve an existing local server by name, running or stopped. `local udf`
/// never creates a server: a name with neither metadata nor a data directory
/// is an error before anything is written. The metadata lock is returned so
/// callers can hold it across their file writes and release it before any
/// HTTP round trip.
fn server_target(name: &str) -> Result<(ServerTarget, MetadataLock)> {
    let lock = server::lock_metadata()?;
    server::recover_current_project_servers_locked(&lock)?;
    let entry = server::server_entry_locked(name, &lock)?;
    let is_postgres = match entry.as_ref().and_then(|entry| entry.info.as_ref()) {
        Some(info) => info.engine != Engine::Clickhouse,
        None => server::is_pg_instance_key(name),
    };
    if is_postgres {
        return Err(Error::UdfServerIsPostgres(name.to_owned()));
    }
    let running = entry
        .filter(|entry| entry.running)
        .and_then(|entry| entry.info);
    let data_dir = server::server_data_dir(name);
    if running.is_none() && !data_dir.is_dir() {
        let project_dir = crate::init::canonical_project_dir()?;
        return Err(Error::UdfServerNotFound {
            name: name.to_owned(),
            project_has_state: looks_like_project_root(&project_dir),
            project_dir,
        });
    }
    Ok((ServerTarget { data_dir, running }, lock))
}

/// Whether `dir` is plausibly where servers are started: it has the
/// `clickhouse/` scaffold or some server's state. Local commands never search
/// parent directories, so a subdirectory of a project has neither.
fn looks_like_project_root(dir: &Path) -> bool {
    if dir.join("clickhouse").is_dir() {
        return true;
    }
    // The metadata lock file is created by any `local udf` call, even in a
    // subdirectory, so it does not count as server state.
    std::fs::read_dir(dir.join(".clickhouse").join("servers"))
        .map(|entries| {
            entries
                .flatten()
                .any(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        })
        .unwrap_or(false)
}

fn require_running<'a>(target: &'a ServerTarget, name: &str) -> Result<&'a ServerInfo> {
    target
        .running
        .as_ref()
        .ok_or_else(|| Error::UdfServerNotRunning(name.to_owned()))
}

/// Why a statement failed: the server could not be reached, or it answered
/// with an error. Deploy tells a rejected definition apart from the rest.
enum QueryError {
    Client(reqwest::Error),
    Unreachable(String),
    Rejected(String),
}

impl QueryError {
    fn into_error(self, info: &ServerInfo, server: &str) -> Error {
        match self {
            Self::Client(error) => error.into(),
            Self::Unreachable(details) => Error::UdfServerUnreachable {
                server: server.to_owned(),
                port: info.http_port,
                details,
            },
            Self::Rejected(details) => Error::UdfQueryFailed {
                server: server.to_owned(),
                details,
            },
        }
    }
}

/// Run one statement over the server's HTTP interface as the default user.
async fn send_query(info: &ServerInfo, sql: &str) -> std::result::Result<String, QueryError> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .build()
        .map_err(QueryError::Client)?;
    let unreachable = |error: reqwest::Error| QueryError::Unreachable(error_chain(&error));
    let response = client
        .post(format!("http://localhost:{}/", info.http_port))
        .body(sql.to_owned())
        .send()
        .await
        .map_err(unreachable)?;
    let status = response.status();
    let body = response.text().await.map_err(unreachable)?;
    if !status.is_success() {
        return Err(QueryError::Rejected(body.trim().to_owned()));
    }
    Ok(body)
}

/// An error's text followed by each of its sources', so a timeout and a
/// refused connection read differently: reqwest's own text only names the
/// URL.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        // Some layers repeat their source's text; keep each part once.
        if !text.ends_with(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

async fn http_query(info: &ServerInfo, server: &str, sql: &str) -> Result<String> {
    send_query(info, sql)
        .await
        .map_err(|error| error.into_error(info, server))
}

async fn loaded_function_names(info: &ServerInfo, server: &str) -> Result<Vec<String>> {
    let body = http_query(info, server, LOADED_FUNCTIONS_SQL).await?;
    Ok(body
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

enum LoadOutcome {
    /// The function is loaded; carries every loaded function's name.
    Loaded(Vec<String>),
    /// The function loaded its new definition, but the reload was rejected
    /// because other deployed functions are broken. ClickHouse loads each
    /// function file on its own, so only those stay unloaded (or keep an
    /// earlier definition), and every later reload on the server fails until
    /// they are fixed or removed.
    LoadedDespite {
        /// Loaded functions known to run their current files.
        current: Vec<String>,
        /// The broken staged functions; empty when the server cannot tell.
        broken: Vec<String>,
        /// ClickHouse's response text.
        details: String,
    },
    NotLoaded,
    /// A reload was rejected and the function's new definition is broken,
    /// or could not be confirmed to have loaded.
    Rejected {
        /// ClickHouse's response text.
        details: String,
        /// The broken staged functions; empty when the server cannot tell.
        blocking: Vec<String>,
    },
}

/// What `deploy` knows about the function it is waiting for.
struct LoadProbe<'a> {
    name: &'a str,
    data_dir: &'a Path,
    /// The server listed the function before this deploy, so seeing it
    /// loaded alone does not show the new definition loaded.
    was_loaded: bool,
    /// The overlay was rewritten by this deploy, so the server may not see
    /// any function file until it rereads its config.
    overlay_fresh: bool,
}

/// Reload, then give the server time to notice a freshly written overlay
/// (its config reloader runs periodically) before concluding the function
/// did not load. A function that has never loaded can pass the first reload
/// and only be rejected by a retry, so every attempt is handled alike.
async fn reload_until_loaded(
    info: &ServerInfo,
    server: &str,
    probe: &LoadProbe<'_>,
) -> Result<LoadOutcome> {
    let deadline = Instant::now() + LOAD_POLL_TIMEOUT;
    let mut pending_rejection = None;
    loop {
        let rejection = match send_query(info, RELOAD_FUNCTIONS_SQL).await {
            Ok(_) => None,
            Err(QueryError::Rejected(details)) => Some(details),
            Err(error) => return Err(error.into_error(info, server)),
        };
        let loaded = loaded_function_names(info, server).await?;
        let is_loaded = loaded.iter().any(|loaded| loaded == probe.name);
        match rejection {
            None if is_loaded => return Ok(LoadOutcome::Loaded(loaded)),
            None => {}
            Some(details) => {
                let report = inspect_rejection(info, probe.data_dir, &details, &loaded).await;
                if let Some(outcome) = rejected_outcome(probe, &report, &loaded, details.clone()) {
                    return Ok(outcome);
                }
                pending_rejection = Some((details, report));
            }
        }
        if Instant::now() >= deadline {
            return Ok(match pending_rejection {
                // Without per-function status, a function still missing
                // after every reload was rejected is most likely broken.
                Some((details, report)) if !report.per_function_status => LoadOutcome::Rejected {
                    details,
                    blocking: report.failed_assuming_active,
                },
                _ => LoadOutcome::NotLoaded,
            });
        }
        tokio::time::sleep(LOAD_POLL_INTERVAL).await;
    }
}

/// What a rejected reload means for the function being deployed, or `None`
/// while the server may simply not see its file yet.
fn rejected_outcome(
    probe: &LoadProbe<'_>,
    report: &RejectionReport,
    loaded: &[String],
    details: String,
) -> Option<LoadOutcome> {
    let name = probe.name;
    let rejected = |blocking: Vec<String>| LoadOutcome::Rejected {
        details: details.clone(),
        blocking,
    };
    if report.failed.iter().any(|failed| failed == name) {
        return Some(rejected(report.failed.clone()));
    }
    if loaded.iter().any(|loaded| loaded == name) {
        // Without per-function status, a function that was loaded before
        // may still run its earlier definition: say so rather than guess.
        if !report.per_function_status && probe.was_loaded {
            return Some(rejected(report.failed.clone()));
        }
        let current = if report.per_function_status {
            loaded
                .iter()
                .filter(|loaded| !report.failed.contains(loaded))
                .cloned()
                .collect()
        } else {
            vec![name.to_owned()]
        };
        return Some(LoadOutcome::LoadedDespite {
            current,
            broken: report.failed.clone(),
            details,
        });
    }
    // Neither loaded nor reported broken: the server has not read the file,
    // unless it evidently reads the overlay's directory, in which case an
    // older server simply cannot name the function as broken.
    if !report.per_function_status && (!probe.overlay_fresh || report.directory_active) {
        return Some(rejected(report.failed_assuming_active.clone()));
    }
    None
}

/// The warning a deploy carries when its function loaded but the reload
/// was rejected because of other functions.
fn reload_still_fails_warning(server: &str, broken: &[String], details: &str) -> String {
    match broken {
        [] => format!(
            "function reloads on server {server} still fail because another deployed UDF is \
             broken; fix and redeploy it, or remove it with `clickhousectl local udf remove`. \
             ClickHouse error: {details}"
        ),
        [one] => format!(
            "function reloads on server {server} still fail because UDF {one} is broken; fix and \
             redeploy it, or remove it with `clickhousectl local udf remove {one} --server \
             {server}`"
        ),
        many => format!(
            "function reloads on server {server} still fail because UDFs {} are broken; fix and \
             redeploy them, or remove each with `clickhousectl local udf remove NAME --server \
             {server}`",
            many.join(", ")
        ),
    }
}

/// One `system.functions` row whose name equals a UDF name but for case.
#[derive(Debug, Clone, PartialEq)]
struct KnownFunction {
    name: String,
    origin: String,
    alias_to: String,
    case_insensitive: bool,
}

/// The `system.functions` rows whose name equals `name` but for case, as
/// `name`, `origin`, `alias_to` and `case_insensitive` columns.
fn functions_named_sql(name: &str) -> String {
    // `name` passed `validate_function_name`, so it is `[A-Za-z0-9_]+` and
    // safe inside a string literal.
    format!(
        "SELECT name, origin, alias_to, case_insensitive FROM system.functions \
         WHERE lower(name) = lower('{name}') FORMAT TabSeparated"
    )
}

fn parse_known_functions(body: &str) -> Vec<KnownFunction> {
    body.lines()
        .filter_map(|line| {
            let mut columns = line.split('\t');
            Some(KnownFunction {
                name: columns.next().filter(|name| !name.is_empty())?.to_owned(),
                origin: columns.next()?.to_owned(),
                alias_to: columns.next()?.to_owned(),
                case_insensitive: columns.next()? == "1",
            })
        })
        .collect()
}

/// The function a UDF named `name` would clash with. ClickHouse refuses an
/// executable UDF whose exact name is taken by a built-in (an alias such as
/// `lcase` has its own row), and a SQL function of the same name shadows it.
/// A differently cased name of a case-insensitive built-in (`LOWER`) loads,
/// but then takes over every query spelling that function in that case, so
/// it counts too. Executable UDFs are ours to replace.
fn name_clash<'a>(name: &str, functions: &'a [KnownFunction]) -> Option<&'a KnownFunction> {
    let clashes = |function: &&KnownFunction| {
        function.origin != "ExecutableUserDefined"
            && (function.name == name
                || (function.case_insensitive && function.name.eq_ignore_ascii_case(name)))
    };
    functions
        .iter()
        .filter(clashes)
        .find(|function| function.name == name)
        .or_else(|| functions.iter().find(clashes))
}

fn name_clash_message(name: &str, server: &str, clash: &KnownFunction) -> String {
    let existing = &clash.name;
    let kind = match clash.origin.as_str() {
        "SQLUserDefined" => {
            return format!(
                "UDF name {name} is taken on server {server} by the SQL function {existing}, \
                 created with CREATE FUNCTION, which would shadow the UDF in every query; run \
                 DROP FUNCTION {existing} or choose another name\n"
            );
        }
        "System" => "built-in function",
        _ => "function",
    };
    if existing != name {
        return format!(
            "UDF name {name} clashes with the case-insensitive {kind} {existing} on server \
             {server}: the UDF would replace it in every query that spells it {name}; choose \
             another name\n"
        );
    }
    let alias = if clash.alias_to.is_empty() {
        String::new()
    } else {
        format!(" (an alias of {})", clash.alias_to)
    };
    format!(
        "UDF name {name} clashes with the {kind} {existing}{alias} on server {server}: \
         ClickHouse would reject the UDF and then fail every function reload; choose another \
         name\n"
    )
}

/// Refuse a UDF name the running server already uses for something other
/// than an executable UDF. Returns whether `name` is a loaded executable UDF.
async fn check_name_is_free(info: &ServerInfo, server: &str, name: &str) -> Result<bool> {
    let functions =
        parse_known_functions(&http_query(info, server, &functions_named_sql(name)).await?);
    match name_clash(name, &functions) {
        Some(clash) => Err(Error::Usage(Box::new(clap::Error::raw(
            clap::error::ErrorKind::ValueValidation,
            name_clash_message(name, server, clash),
        )))),
        None => Ok(functions
            .iter()
            .any(|function| function.name == name && function.origin == "ExecutableUserDefined")),
    }
}

/// What a rejected reload says about the functions staged on this server.
/// Only staged functions count, so every name maps back to files `remove`
/// can delete.
#[derive(Debug, Default)]
struct RejectionReport {
    /// Staged functions known to be broken; empty when none is identified.
    failed: Vec<String>,
    /// `failed`, plus every staged function the server does not list even
    /// where it may not see the function directory yet.
    failed_assuming_active: Vec<String>,
    /// The server reports per-function load status (ClickHouse 26.2+).
    per_function_status: bool,
    /// Some staged function is loaded, so the server reads the directory.
    directory_active: bool,
}

/// Identify the broken functions behind a rejected reload. ClickHouse loads
/// each function file on its own, so the rest still load. 26.2 and later
/// report each function's load status. Older servers name a function only in
/// some error texts, so a staged function the server does not list is taken
/// as broken once the server evidently reads the function directory.
async fn inspect_rejection(
    info: &ServerInfo,
    data_dir: &Path,
    details: &str,
    loaded: &[String],
) -> RejectionReport {
    let staged: Vec<String> = staged_udfs(data_dir)
        .unwrap_or_default()
        .into_iter()
        .map(|staged| staged.name)
        .collect();
    let status = send_query(info, FAILED_FUNCTIONS_SQL).await.ok();
    rejection_report(&staged, status.as_deref(), details, loaded)
}

/// [`inspect_rejection`]'s logic, given the staged names, the per-function
/// status query's answer (`None` when the server lacks the table), the
/// reload's error text and the loaded names.
fn rejection_report(
    staged: &[String],
    status: Option<&str>,
    details: &str,
    loaded: &[String],
) -> RejectionReport {
    let mut reported = culprits_in_error_text(details);
    if let Some(body) = status {
        reported.extend(
            body.lines()
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
    }
    let directory_active = staged.iter().any(|name| loaded.contains(name));
    let unlisted = |name: &String| status.is_none() && !loaded.contains(name);
    let pick = |assume_active: bool| -> Vec<String> {
        staged
            .iter()
            .filter(|name| {
                reported.contains(name) || ((assume_active || directory_active) && unlisted(name))
            })
            .cloned()
            .collect()
    };
    RejectionReport {
        failed: pick(false),
        failed_assuming_active: pick(true),
        per_function_status: status.is_some(),
        directory_active,
    }
}

/// The broken functions behind a rejected `reload` or `remove` reload. The
/// server already had the chance to read the function directory, so on an
/// older server every staged function it does not list counts.
async fn blocking_functions(info: &ServerInfo, data_dir: &Path, details: &str) -> Vec<String> {
    let loaded: Option<Vec<String>> =
        send_query(info, LOADED_FUNCTIONS_SQL)
            .await
            .ok()
            .map(|body| {
                body.lines()
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect()
            });
    let report = inspect_rejection(
        info,
        data_dir,
        details,
        loaded.as_deref().unwrap_or_default(),
    )
    .await;
    // Without the loaded list, no function can be told apart as unlisted.
    if loaded.is_some() {
        report.failed_assuming_active
    } else {
        report.failed
    }
}

/// Function names a rejected reload's error text identifies. Only a name
/// clash names its function (`The function 'lower' already exists`); other
/// rejections, such as an unknown type, name neither function nor file.
fn culprits_in_error_text(details: &str) -> Vec<String> {
    const PREFIXES: [&str; 2] = ["The function '", "The aggregate function '"];
    const SUFFIX: &str = "' already exists";
    let mut names = Vec::new();
    for prefix in PREFIXES {
        let mut rest = details;
        while let Some(start) = rest.find(prefix) {
            rest = &rest[start + prefix.len()..];
            let Some(end) = rest.find('\'') else { break };
            if rest[end..].starts_with(SUFFIX) {
                names.push(rest[..end].to_owned());
            }
            rest = &rest[end..];
        }
    }
    names
}

// ── list / remove / reload ──────────────────────────────────────────────────

async fn list(server_name: &str, json: bool) -> Result<()> {
    let (target, lock) = server_target(server_name)?;
    drop(lock);
    let loaded = match &target.running {
        Some(info) => Some(loaded_function_names(info, server_name).await?),
        None => None,
    };
    let udfs = staged_udfs(&target.data_dir)?
        .into_iter()
        .map(|staged| {
            let is_loaded = loaded.as_ref().map(|loaded| loaded.contains(&staged.name));
            UdfListEntry {
                last_deploy_rejected: last_deploy_rejected(
                    &target.data_dir,
                    &staged.name,
                    target.running.as_ref(),
                    is_loaded,
                ),
                loaded: is_loaded,
                name: staged.name,
                r#type: staged.kind,
                runtime: staged.runtime,
            }
        })
        .collect();
    let out = UdfListOutput {
        server: server_name.to_owned(),
        server_running: target.running.is_some(),
        udfs,
    };
    output::print_output(&out, json);
    Ok(())
}

async fn remove(name: &str, server_name: &str, json: bool) -> Result<()> {
    let (target, lock) = server_target(server_name)?;
    let mut removed = false;
    for file in [
        function_xml_path(&target.data_dir, name),
        sidecar_path(&target.data_dir, name),
    ] {
        if file.is_file() {
            std::fs::remove_file(file)?;
            removed = true;
        }
    }
    let scripts_dir = target.data_dir.join(SCRIPTS_DIR).join(name);
    if scripts_dir.is_dir() {
        std::fs::remove_dir_all(scripts_dir)?;
        removed = true;
    }
    remove_if_present(&rejected_marker_path(&target.data_dir, name))?;
    if !removed {
        return Err(Error::UdfNotFound {
            name: name.to_owned(),
            server: server_name.to_owned(),
        });
    }
    write_overlay(&target.data_dir)?;
    drop(lock);
    let reloaded = match &target.running {
        Some(info) => {
            match send_query(info, RELOAD_FUNCTIONS_SQL).await {
                Ok(_) => {}
                Err(QueryError::Rejected(details)) => {
                    return Err(Error::UdfReloadBlocked {
                        removed: Some(name.to_owned()),
                        server: server_name.to_owned(),
                        blocking: blocking_functions(info, &target.data_dir, &details).await,
                        details,
                    });
                }
                Err(error) => return Err(error.into_error(info, server_name)),
            }
            let loaded = loaded_function_names(info, server_name).await?;
            clear_rejected_markers(&target.data_dir, &loaded)?;
            true
        }
        None => false,
    };
    let out = UdfRemoveOutput {
        name: name.to_owned(),
        server: server_name.to_owned(),
        reloaded,
    };
    output::print_output(&out, json);
    Ok(())
}

async fn reload(server_name: &str, json: bool) -> Result<()> {
    let (target, lock) = server_target(server_name)?;
    drop(lock);
    let info = require_running(&target, server_name)?;
    match send_query(info, RELOAD_FUNCTIONS_SQL).await {
        Ok(_) => {}
        Err(QueryError::Rejected(details)) => {
            return Err(Error::UdfReloadBlocked {
                removed: None,
                server: server_name.to_owned(),
                blocking: blocking_functions(info, &target.data_dir, &details).await,
                details,
            });
        }
        Err(error) => return Err(error.into_error(info, server_name)),
    }
    let loaded = loaded_function_names(info, server_name).await?;
    clear_rejected_markers(&target.data_dir, &loaded)?;
    let out = UdfReloadOutput {
        server: server_name.to_owned(),
        loaded,
    };
    output::print_output(&out, json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn definition(kind: LocalUdfType, runtime: LocalUdfRuntime) -> LocalUdfDefinition {
        LocalUdfDefinition {
            function_name: "my_fn".into(),
            kind,
            runtime,
            arguments: vec![LocalUdfArgument {
                name: "value".into(),
                kind: "String".into(),
            }],
            return_type: "String".into(),
            return_name: None,
            format: None,
            command_read_timeout: None,
            command_write_timeout: None,
            max_command_execution_time: None,
            pool_size: None,
            send_chunk_header: None,
            deterministic: None,
            memory_limit_mib: None,
            sandbox_type: None,
            sandbox_version: None,
        }
    }

    fn python_command_fixture() -> ResolvedCommand {
        ResolvedCommand {
            command: "'/usr/bin/python3.11' '/data/user_scripts/my_fn/main.py'".into(),
            execute_direct: false,
        }
    }

    #[test]
    fn definition_template_passes_shared_validation_for_every_variant() {
        for (runtime, kind) in [
            (UdfRuntimeArg::Python311, UdfTypeArg::Executable),
            (UdfRuntimeArg::Python311, UdfTypeArg::ExecutablePool),
            (UdfRuntimeArg::Native, UdfTypeArg::Executable),
            (UdfRuntimeArg::Native, UdfTypeArg::ExecutablePool),
        ] {
            let text = definition_template("my_fn", runtime, kind);
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(
                crate::udf::validate_definition(&value, true).unwrap(),
                kind.as_str()
            );
            assert!(crate::udf::check_function_name(&value, "my_fn").is_ok());
            let typed: LocalUdfDefinition = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(typed.function_name, "my_fn");
            assert_eq!(typed.ignored_fields(), Vec::<String>::new());
            assert_eq!(value["runtime"], runtime.as_str());
            assert!(text.ends_with("}\n"));
        }
    }

    #[test]
    fn python_template_starts_with_a_shebang_and_mentions_the_function() {
        let python = python_template("my_fn");
        assert!(python.starts_with("#!/usr/bin/env python3\n"));
        assert!(python.contains("\"\"\"my_fn: executable UDF entrypoint."));
        assert!(python.contains("sys.stdout.flush()"));
        assert!(python.contains(r"(\t, \n, \\)"));
        assert!(python.contains("    return value\n"));
        assert!(native_next_step().starts_with("Build linux/amd64 and linux/arm64 binaries"));
    }

    #[test]
    fn case_only_clash_finds_a_differently_cased_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            case_only_clash(&tmp.path().join("missing"), "rev").unwrap(),
            None
        );
        std::fs::create_dir(tmp.path().join("rev")).unwrap();
        assert_eq!(case_only_clash(tmp.path(), "rev").unwrap(), None);
        assert_eq!(case_only_clash(tmp.path(), "other").unwrap(), None);
        assert_eq!(
            case_only_clash(tmp.path(), "Rev").unwrap().as_deref(),
            Some("rev")
        );
    }

    #[test]
    fn write_if_absent_creates_once_and_keeps_existing_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("main.py");
        let mut created = Vec::new();

        write_if_absent(tmp.path(), "main.py", "first\n", 0o755, &mut created).unwrap();
        assert_eq!(created, vec!["main.py"]);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        write_if_absent(tmp.path(), "main.py", "second\n", 0o755, &mut created).unwrap();
        assert_eq!(created, vec!["main.py"]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first\n");
    }

    #[test]
    fn local_definition_requires_the_directory_name_to_match() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_fn");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("udf.json"),
            definition_template("my_fn", UdfRuntimeArg::Python311, UdfTypeArg::Executable),
        )
        .unwrap();
        let (typed, ignored) = load_local_definition(&dir, "my_fn").unwrap();
        assert_eq!(typed.function_name, "my_fn");
        assert!(ignored.is_empty());

        let error = load_local_definition(&dir, "other").unwrap_err();
        assert!(matches!(
            &error,
            Error::UdfDefinitionInvalid { path, reason }
                if path == &dir.join("udf.json")
                    && reason == "functionName is my_fn, but the command targets other"
        ));
    }

    #[test]
    fn local_definition_matches_the_library_request_schema_in_both_directions() {
        use clickhouse_cloud_api::models::*;
        // Every field the library can send must be known locally.
        let maximal = UdfCreateRequestV2 {
            deterministic: Some(true),
            memory_limit_mib: Some(256),
            arguments: vec![UdfArgument {
                name: "x".into(),
                r#type: "UInt64".into(),
            }],
            command_read_timeout: Some(1000),
            command_write_timeout: Some(2000),
            format: Some("JSONEachRow".into()),
            function_name: "my_fn".into(),
            max_command_execution_time: Some(5),
            pool_size: Some(4),
            return_name: Some("result".into()),
            return_type: "UInt64".into(),
            runtime: UdfRuntime::Python3_11,
            sandbox_type: Some(UdfSandboxType::Netenable),
            sandbox_version: Some(UdfSandboxVersion::V3),
            send_chunk_header: Some(true),
            r#type: UdfCreateRequestV2Type::ExecutablePool,
            upload_id: "upload".into(),
        };
        let mut value = serde_json::to_value(&maximal).unwrap();
        value.as_object_mut().unwrap().remove("uploadId");
        let local: LocalUdfDefinition = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(local.pool_size, Some(4));
        assert_eq!(local.runtime, LocalUdfRuntime::Python311);
        assert_eq!(
            local.ignored_fields(),
            vec!["memoryLimitMib", "sandboxType", "sandboxVersion"]
        );
        // And every field the local type carries must be accepted strictly by
        // the library request, so a local definition always deploys to Cloud.
        let mut round_trip = serde_json::to_value(&local).unwrap();
        round_trip["uploadId"] = json!("upload");
        let request: UdfCreateRequestV2 =
            crate::cloud::config::deserialize_strict_config(round_trip, "UDF definition").unwrap();
        assert_eq!(request, maximal);
    }

    #[test]
    fn ignored_fields_lists_pool_settings_only_on_a_plain_executable() {
        let mut def = definition(LocalUdfType::Executable, LocalUdfRuntime::Python311);
        def.pool_size = Some(4);
        assert_eq!(def.ignored_fields(), vec!["poolSize"]);
        def.max_command_execution_time = Some(7);
        def.memory_limit_mib = Some(256);
        assert_eq!(
            def.ignored_fields(),
            vec!["poolSize", "maxCommandExecutionTime", "memoryLimitMib"]
        );
        def.kind = LocalUdfType::ExecutablePool;
        assert_eq!(def.ignored_fields(), vec!["memoryLimitMib"]);
    }

    #[test]
    fn local_function_names_fit_every_derived_file_name() {
        let longest = "a".repeat(MAX_LOCAL_NAME_LEN);
        assert!(validate_local_function_name(&longest).is_ok());
        let too_long = "a".repeat(MAX_LOCAL_NAME_LEN + 1);
        assert_eq!(
            validate_local_function_name(&too_long).unwrap_err(),
            format!(
                "Use at most {MAX_LOCAL_NAME_LEN} characters; this name has {}",
                MAX_LOCAL_NAME_LEN + 1
            )
        );
        // The shared pattern still applies first.
        assert!(validate_local_function_name("1abc").is_err());

        // Every file the CLI names after a maximal function can be created.
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path();
        std::fs::create_dir_all(data_dir.join(FUNCTIONS_DIR)).unwrap();
        for path in [
            function_xml_path(data_dir, &longest),
            sidecar_path(data_dir, &longest),
            rejected_marker_path(data_dir, &longest),
        ] {
            std::fs::write(&path, "").unwrap();
        }
        let source = tmp.path().join("main.py");
        std::fs::write(&source, "").unwrap();
        let entries = [SourceEntry {
            relative: PathBuf::from(PYTHON_ENTRYPOINT),
            absolute: source,
            is_dir: false,
            mode: 0o644,
        }];
        // Stages through `.<name>.staging-<pid>`; the worst case is a 10-digit pid.
        let scripts = data_dir.join(SCRIPTS_DIR).join(&longest);
        stage_scripts(&entries, &scripts, &[]).unwrap();
        assert!(scripts.join(PYTHON_ENTRYPOINT).is_file());
        let worst = data_dir
            .join(SCRIPTS_DIR)
            .join(format!(".{longest}.staging-{}", u32::MAX));
        std::fs::create_dir(&worst).unwrap();
    }

    #[test]
    fn local_definition_rejects_unknown_fields_and_wrong_types() {
        let mut value: serde_json::Value = serde_json::from_str(&definition_template(
            "my_fn",
            UdfRuntimeArg::Python311,
            UdfTypeArg::Executable,
        ))
        .unwrap();
        value["typo"] = json!(1);
        assert!(serde_json::from_value::<LocalUdfDefinition>(value.clone()).is_err());
        value.as_object_mut().unwrap().remove("typo");
        value["arguments"][0]["extra"] = json!(1);
        assert!(serde_json::from_value::<LocalUdfDefinition>(value.clone()).is_err());
        value["arguments"][0]
            .as_object_mut()
            .unwrap()
            .remove("extra");
        value["poolSize"] = json!("4");
        assert!(serde_json::from_value::<LocalUdfDefinition>(value.clone()).is_err());
        value["poolSize"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<LocalUdfDefinition>(value).is_ok());
    }

    #[test]
    fn render_function_xml_python_executable_emits_shell_command() {
        let xml = render_function_xml(
            &definition(LocalUdfType::Executable, LocalUdfRuntime::Python311),
            &python_command_fixture(),
        );
        assert_eq!(
            xml,
            "<clickhouse>\n    <function>\n        <type>executable</type>\n        <name>my_fn</name>\n        <command>'/usr/bin/python3.11' '/data/user_scripts/my_fn/main.py'</command>\n        <execute_direct>0</execute_direct>\n        <format>TabSeparated</format>\n        <argument>\n            <type>String</type>\n            <name>value</name>\n        </argument>\n        <return_type>String</return_type>\n    </function>\n</clickhouse>\n"
        );
    }

    #[test]
    fn render_function_xml_native_pool_emits_pool_fields_and_direct_command() {
        let mut def = definition(LocalUdfType::ExecutablePool, LocalUdfRuntime::Native);
        def.pool_size = Some(4);
        def.max_command_execution_time = Some(7);
        def.command_read_timeout = Some(1000);
        def.command_write_timeout = Some(2000);
        def.send_chunk_header = Some(true);
        def.deterministic = Some(false);
        def.return_name = Some("result".into());
        def.format = Some("JSONEachRow".into());
        let xml = render_function_xml(
            &def,
            &ResolvedCommand {
                command: "my_fn/amd64/main".into(),
                execute_direct: true,
            },
        );
        for expected in [
            "<type>executable_pool</type>",
            "<command>my_fn/amd64/main</command>",
            "<execute_direct>1</execute_direct>",
            "<format>JSONEachRow</format>",
            "<return_name>result</return_name>",
            "<command_read_timeout>1000</command_read_timeout>",
            "<command_write_timeout>2000</command_write_timeout>",
            "<pool_size>4</pool_size>",
            "<max_command_execution_time>7</max_command_execution_time>",
            "<send_chunk_header>true</send_chunk_header>",
            "<deterministic>false</deterministic>",
        ] {
            assert!(xml.contains(expected), "{expected} missing in {xml}");
        }

        // Pool-only fields are dropped for plain executables.
        def.kind = LocalUdfType::Executable;
        let xml = render_function_xml(&def, &python_command_fixture());
        assert!(!xml.contains("pool_size"));
        assert!(!xml.contains("max_command_execution_time"));
    }

    #[test]
    fn render_function_xml_escapes_markup_in_types_names_and_commands() {
        let mut def = definition(LocalUdfType::Executable, LocalUdfRuntime::Python311);
        def.arguments = vec![LocalUdfArgument {
            name: "pairs".into(),
            kind: "Map(String, Array<UInt8>)".into(),
        }];
        def.return_type = "Tuple(a UInt8, b String)".into();
        def.return_name = Some("a_and_b".into());
        let xml = render_function_xml(
            &def,
            &ResolvedCommand {
                command: "'/tmp/a&b/python3' '/tmp/<x>/main.py'".into(),
                execute_direct: false,
            },
        );
        assert!(xml.contains("<type>Map(String, Array&lt;UInt8&gt;)</type>"));
        assert!(xml.contains("<command>'/tmp/a&amp;b/python3' '/tmp/&lt;x&gt;/main.py'</command>"));
        assert!(xml.contains("<return_type>Tuple(a UInt8, b String)</return_type>"));
        assert_eq!(xml_escape("a&b<c>d\"e'f"), "a&amp;b&lt;c&gt;d\"e'f");
    }

    #[test]
    fn overlay_points_at_absolute_function_glob_and_scripts_dir() {
        let xml = render_overlay_xml(Path::new("/work/.clickhouse/servers/default/data"));
        assert!(xml.contains(
            "<user_defined_executable_functions_config>/work/.clickhouse/servers/default/data/user_defined_functions/*_function.xml</user_defined_executable_functions_config>"
        ));
        assert!(xml.contains(
            "<user_scripts_path>/work/.clickhouse/servers/default/data/user_scripts/</user_scripts_path>"
        ));
        assert!(xml.starts_with("<clickhouse>\n"));
        assert!(xml.ends_with("</clickhouse>\n"));

        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let path = write_overlay(&data_dir).unwrap();
        assert_eq!(path, data_dir.join("config.d").join(OVERLAY_FILE));
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("user_defined_functions/*_function.xml"));
        assert!(
            std::fs::read_dir(data_dir.join("config.d"))
                .unwrap()
                .count()
                == 1,
            "no temporary file left behind"
        );
    }

    #[test]
    fn overlay_is_rewritten_only_when_its_content_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let path = write_overlay(&data_dir).unwrap();
        let past = SystemTime::now() - Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(past)
            .unwrap();

        write_overlay(&data_dir).unwrap();
        assert_eq!(
            modified_at(&path),
            Some(past),
            "identical overlay rewritten"
        );

        std::fs::write(&path, "<clickhouse/>\n").unwrap();
        write_overlay(&data_dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            render_overlay_xml(&data_dir.canonicalize().unwrap())
        );
    }

    #[test]
    fn native_arch_is_linux_amd64_or_arm64_only() {
        assert_eq!(native_arch_for("linux", "x86_64").unwrap(), "amd64");
        assert_eq!(native_arch_for("linux", "aarch64").unwrap(), "arm64");
        for (os, arch) in [
            ("macos", "aarch64"),
            ("macos", "x86_64"),
            ("windows", "x86_64"),
        ] {
            let error = native_arch_for(os, arch).unwrap_err();
            assert!(
                matches!(error, Error::UdfRuntimeUnsupported(_)),
                "{os}/{arch}"
            );
            assert_eq!(error.to_string(), NATIVE_UNSUPPORTED);
        }
        let error = native_arch_for("linux", "riscv64").unwrap_err();
        assert!(matches!(error, Error::UdfRuntimeUnsupported(_)));
        assert!(error.to_string().contains("riscv64"), "{error}");
    }

    #[test]
    fn requirements_file_is_reported_as_ignored_for_python_only() {
        let entry = |relative: &str| SourceEntry {
            relative: PathBuf::from(relative),
            absolute: PathBuf::from("/src").join(relative),
            is_dir: false,
            mode: 0o644,
        };
        let tmp = tempfile::tempdir().unwrap();
        let entries = vec![entry("main.py"), entry("requirements.txt")];
        assert_eq!(
            ignored_files(tmp.path(), &entries, UdfRuntimeKind::Python311, None),
            vec!["requirements.txt"]
        );
        assert!(
            ignored_files(tmp.path(), &entries[..1], UdfRuntimeKind::Python311, None).is_empty()
        );
        assert!(
            ignored_files(tmp.path(), &entries, UdfRuntimeKind::Native, Some("amd64")).is_empty()
        );
    }

    #[test]
    fn native_reports_the_other_architecture_directory_as_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        for arch in NATIVE_ARCH_DIRS {
            std::fs::create_dir_all(tmp.path().join(arch)).unwrap();
        }
        assert_eq!(
            ignored_files(tmp.path(), &[], UdfRuntimeKind::Native, Some("amd64")),
            vec!["arm64/"]
        );
        assert_eq!(
            ignored_files(tmp.path(), &[], UdfRuntimeKind::Native, Some("arm64")),
            vec!["amd64/"]
        );
        std::fs::remove_dir(tmp.path().join("arm64")).unwrap();
        assert!(ignored_files(tmp.path(), &[], UdfRuntimeKind::Native, Some("amd64")).is_empty());
    }

    #[test]
    fn python_command_execs_the_interpreter_so_no_shell_stays_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let python = tmp.path().join("python3");
        std::fs::write(&python, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (command, interpreter) =
            python_command("my_fn", Path::new("/data"), Some(&python)).unwrap();
        assert_eq!(
            command.command,
            format!(
                "exec '{}' '/data/user_scripts/my_fn/main.py'",
                python.display()
            )
        );
        assert!(!command.execute_direct);
        assert_eq!(interpreter, Some(python));
    }

    #[test]
    fn explicit_relative_python_is_made_absolute_and_normalised() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = std::env::current_dir().unwrap();
        let python = tmp.path().join("py3");
        std::fs::write(&python, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj")).unwrap();
        // A path relative to the cwd that walks through `proj/..`.
        let relative = up_to_root(&cwd).join(tmp.path().strip_prefix("/").unwrap());
        let explicit = relative.join("proj/../py3");
        let resolved = resolve_python(Some(&explicit), OsStr::new("")).unwrap();
        assert_eq!(resolved, python);
        assert!(resolved.is_absolute());

        assert_eq!(
            absolute(PathBuf::from("/a/./b/../c")).unwrap(),
            PathBuf::from("/a/c")
        );
    }

    /// `../..` up from `dir` to the filesystem root.
    fn up_to_root(dir: &Path) -> PathBuf {
        dir.components()
            .skip(1)
            .map(|_| std::path::Component::ParentDir)
            .collect()
    }

    #[test]
    fn python_version_warning_flags_anything_but_3_11() {
        let path = Path::new("/usr/bin/python3");
        assert_eq!(python_version_warning(path, Some("3.11")), None);
        let warning = python_version_warning(path, Some("3.12")).unwrap();
        assert!(
            warning.contains("/usr/bin/python3") && warning.contains("3.12"),
            "{warning}"
        );
        let warning = python_version_warning(path, None).unwrap();
        assert!(warning.contains("/usr/bin/python3"), "{warning}");
    }

    #[tokio::test]
    async fn python_version_reads_major_minor_and_rejects_anything_else() {
        let tmp = tempfile::tempdir().unwrap();
        let make = |name: &str, body: &str| {
            let path = tmp.path().join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        };
        assert_eq!(
            python_version(&make("ok", "echo 3.12")).await.as_deref(),
            Some("3.12")
        );
        assert_eq!(
            python_version(&make("failing", "echo 3.11; exit 1")).await,
            None
        );
        assert_eq!(python_version(&make("garbage", "echo hello")).await, None);
        assert_eq!(python_version(&tmp.path().join("missing")).await, None);
    }

    #[test]
    fn resolve_python_prefers_explicit_then_3_11_then_3_on_path() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        let other = tmp.path().join("other");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        let make = |dir: &Path, name: &str| {
            let path = dir.join(name);
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        };
        let path_var = std::env::join_paths([&other, &bin]).unwrap();

        let error = resolve_python(None, &path_var).unwrap_err();
        assert!(matches!(error, Error::UdfInterpreterNotFound(_)));
        assert!(error.to_string().contains("pass --python"), "{error}");

        let python3 = make(&bin, "python3");
        assert_eq!(resolve_python(None, &path_var).unwrap(), python3);
        let python311 = make(&other, "python3.11");
        assert_eq!(resolve_python(None, &path_var).unwrap(), python311);

        // A non-executable candidate is skipped.
        std::fs::set_permissions(&python311, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(resolve_python(None, &path_var).unwrap(), python3);

        let explicit = make(tmp.path(), "custom-python");
        assert_eq!(
            resolve_python(Some(&explicit), &path_var).unwrap(),
            explicit
        );
        assert_eq!(
            resolve_python(Some(Path::new("python3")), &path_var).unwrap(),
            python3
        );
        let error = resolve_python(Some(Path::new("/nope/python")), &path_var).unwrap_err();
        assert!(error.to_string().contains("/nope/python"), "{error}");
    }

    #[test]
    fn shell_quote_single_quotes_paths_and_rejects_embedded_quotes() {
        assert_eq!(
            shell_quote(
                Path::new("/opt/my python/bin/python3"),
                UdfCommandPath::Interpreter
            )
            .unwrap(),
            "'/opt/my python/bin/python3'"
        );
        assert!(matches!(
            shell_quote(Path::new("/opt/it's/python3"), UdfCommandPath::Interpreter).unwrap_err(),
            Error::UdfPathUnquotable {
                role: UdfCommandPath::Interpreter,
                ..
            }
        ));
    }

    #[test]
    fn python_command_names_the_path_that_holds_the_quote() {
        let tmp = tempfile::tempdir().unwrap();
        let clean = tmp.path().join("bin/python3");
        let quoted = tmp.path().join("it's/python3");
        for path in [&clean, &quoted] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let data_dir = tmp.path().join("o'brien/.clickhouse/servers/default/data");
        let Err(error) = python_command("my_fn", &data_dir, Some(&clean)) else {
            panic!("a quoted staged script path must be rejected");
        };
        assert_eq!(
            error.to_string(),
            format!(
                "Staged script path '{}' contains a single quote or invalid UTF-8, which the \
                 server's shell command cannot carry; move the project to a directory whose \
                 path has none",
                data_dir.join("user_scripts/my_fn/main.py").display()
            )
        );

        let clean_data = tmp.path().join("data");
        let Err(error) = python_command("my_fn", &clean_data, Some(&quoted)) else {
            panic!("a quoted interpreter path must be rejected");
        };
        assert_eq!(
            error.to_string(),
            format!(
                "Python interpreter path '{}' contains a single quote or invalid UTF-8, which \
                 the server's shell command cannot carry; pass --python with a path that has \
                 none",
                quoted.display()
            )
        );
    }

    #[test]
    fn stage_scripts_replaces_the_previous_tree_and_sets_the_entrypoint_bits() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("src");
        for arch in NATIVE_ARCH_DIRS {
            std::fs::create_dir_all(source.join(arch)).unwrap();
            std::fs::write(source.join(arch).join("main"), "binary\n").unwrap();
        }
        std::fs::write(source.join("Cargo.toml"), "[package]\n").unwrap();
        let entries = crate::udf::collect_source_entries(&source, UdfRuntimeKind::Native).unwrap();

        let target = tmp.path().join("user_scripts").join("my_fn");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("stale.txt"), "old\n").unwrap();

        let executables: Vec<PathBuf> = NATIVE_ARCH_DIRS
            .iter()
            .map(|arch| Path::new(arch).join(NATIVE_ENTRYPOINT))
            .collect();
        stage_scripts(&entries, &target, &executables).unwrap();
        for arch in NATIVE_ARCH_DIRS {
            let main = target.join(arch).join("main");
            assert!(main.is_file());
            assert_eq!(
                std::fs::metadata(&main).unwrap().permissions().mode() & 0o777,
                0o755
            );
        }
        assert!(!target.join("stale.txt").exists());
        assert!(!target.join("Cargo.toml").exists());
        let leftovers: Vec<_> = std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, vec!["my_fn"]);
    }

    #[test]
    fn rewritten_function_files_always_move_past_the_previous_second() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("my_fn_function.xml");
        write_atomic(&path, "<clickhouse/>").unwrap();
        let previous = modified_at(&path).unwrap();
        let whole_seconds = |time: SystemTime| {
            time.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        };

        // Same second: the rewrite is pushed one second past the old file.
        write_atomic(&path, "<clickhouse></clickhouse>").unwrap();
        bump_mtime_past(&path, Some(previous)).unwrap();
        assert!(whole_seconds(modified_at(&path).unwrap()) > whole_seconds(previous));

        // Already newer: left alone.
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let before = modified_at(&path).unwrap();
        bump_mtime_past(&path, Some(old)).unwrap();
        assert_eq!(modified_at(&path).unwrap(), before);

        // First write: nothing to compare against.
        bump_mtime_past(&path, None).unwrap();
        bump_mtime_past(&tmp.path().join("missing"), Some(previous)).unwrap();
    }

    #[test]
    fn staged_udfs_reads_function_files_and_sidecars() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        assert!(staged_udfs(&data_dir).unwrap().is_empty());

        std::fs::create_dir_all(data_dir.join(FUNCTIONS_DIR)).unwrap();
        let def = definition(LocalUdfType::ExecutablePool, LocalUdfRuntime::Native);
        write_atomic(&function_xml_path(&data_dir, "my_fn"), "<clickhouse/>").unwrap();
        write_atomic(
            &sidecar_path(&data_dir, "my_fn"),
            &serde_json::to_string(&def).unwrap(),
        )
        .unwrap();
        write_atomic(&function_xml_path(&data_dir, "handmade"), "<clickhouse/>").unwrap();
        std::fs::write(data_dir.join(FUNCTIONS_DIR).join("notes.txt"), "").unwrap();

        let staged = staged_udfs(&data_dir).unwrap();
        let listed: Vec<(String, Option<String>, Option<String>)> = staged
            .into_iter()
            .map(|udf| (udf.name, udf.kind, udf.runtime))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("handmade".to_string(), None, None),
                (
                    "my_fn".to_string(),
                    Some("executable_pool".to_string()),
                    Some("native".to_string())
                ),
            ]
        );
    }

    #[test]
    fn culprits_in_error_text_finds_only_a_clashing_function_name() {
        // Texts returned by `SYSTEM RELOAD FUNCTIONS` on ClickHouse 25.12 and 26.9.
        let clash = "Code: 609. DB::Exception: The function 'lower' already exists. \
                     (FUNCTION_ALREADY_EXISTS) (version 25.12.10.7 (official build))";
        assert_eq!(culprits_in_error_text(clash), vec!["lower".to_owned()]);
        let bad_type = "Code: 50. DB::Exception: Unknown data type family: NotAType. \
                        (UNKNOWN_TYPE) (version 26.9.1.1312 (official build))";
        assert!(culprits_in_error_text(bad_type).is_empty());
        let missing_field = "Poco::Exception. Code: 1000, e.code() = 0, Not found: \
                             function.return_type (version 26.9.1.1312 (official build))";
        assert!(culprits_in_error_text(missing_field).is_empty());
        assert!(culprits_in_error_text("The function 'unterminated").is_empty());
        assert!(culprits_in_error_text("The function 'f' is odd").is_empty());
        let aggregate = "Code: 609. DB::Exception: The aggregate function 'sum' already exists. \
                         (FUNCTION_ALREADY_EXISTS) (version 26.9.1.1312 (official build))";
        assert_eq!(culprits_in_error_text(aggregate), vec!["sum".to_owned()]);
    }

    fn known(name: &str, origin: &str, alias_to: &str, case_insensitive: bool) -> KnownFunction {
        KnownFunction {
            name: name.into(),
            origin: origin.into(),
            alias_to: alias_to.into(),
            case_insensitive,
        }
    }

    #[test]
    fn known_functions_parse_from_tab_separated_rows() {
        assert!(functions_named_sql("my_fn").contains("lower(name) = lower('my_fn')"));
        assert_eq!(
            parse_known_functions(
                "lcase\tSystem\tlower\t1\nmy_fn\tExecutableUserDefined\t\t0\n\nbad\n"
            ),
            vec![
                known("lcase", "System", "lower", true),
                known("my_fn", "ExecutableUserDefined", "", false),
            ]
        );
    }

    #[test]
    fn name_clash_covers_builtins_aliases_case_and_sql_functions_only() {
        let functions = [
            known("lower", "System", "", true),
            known("lcase", "System", "lower", true),
            known("toString", "System", "", false),
            known("sql_fn", "SQLUserDefined", "", false),
            known("my_fn", "ExecutableUserDefined", "", false),
        ];
        let clash = |name| name_clash(name, &functions).map(|function| function.name.as_str());
        assert_eq!(clash("lower"), Some("lower"));
        assert_eq!(clash("LOWER"), Some("lower"));
        assert_eq!(clash("lcase"), Some("lcase"));
        assert_eq!(clash("sql_fn"), Some("sql_fn"));
        assert_eq!(clash("toString"), Some("toString"));
        assert_eq!(clash("TOSTRING"), None);
        assert_eq!(clash("SQL_FN"), None);
        assert_eq!(clash("my_fn"), None);
        assert_eq!(clash("rev"), None);
        // An exact match wins over a case-insensitive one.
        let both = [
            known("Abc", "System", "", true),
            known("abc", "System", "", false),
        ];
        assert_eq!(name_clash("abc", &both).unwrap().name, "abc");

        let message = |name, function| name_clash_message(name, "default", function);
        assert!(
            message("lcase", &functions[1]).contains("lcase (an alias of lower) on server default")
        );
        assert!(
            message("LOWER", &functions[0])
                .contains("case-insensitive built-in function lower on server default")
        );
        assert!(message("sql_fn", &functions[3]).contains("DROP FUNCTION sql_fn"));
        assert!(message("lower", &functions[0]).ends_with('\n'));
    }

    #[test]
    fn rejected_marker_marks_stale_until_a_later_server_start_loads_the_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(FUNCTIONS_DIR)).unwrap();
        let info = |started_at: &str| ServerInfo {
            name: "default".into(),
            pid: 1,
            version: "26.9.1.1312".into(),
            http_port: 8123,
            tcp_port: 9000,
            started_at: started_at.into(),
            cwd: String::new(),
            engine: Engine::Clickhouse,
            container_id: None,
        };
        let early = info("1000");
        assert!(!last_deploy_rejected(
            tmp.path(),
            "my_fn",
            Some(&early),
            Some(true)
        ));

        std::fs::write(rejected_marker_path(tmp.path(), "my_fn"), "").unwrap();
        for loaded in [Some(true), Some(false), None] {
            assert!(last_deploy_rejected(
                tmp.path(),
                "my_fn",
                Some(&early),
                loaded
            ));
        }
        assert!(last_deploy_rejected(tmp.path(), "my_fn", None, None));
        assert!(last_deploy_rejected(
            tmp.path(),
            "my_fn",
            Some(&info("recovered")),
            Some(true)
        ));
        let later = info(&(u64::MAX / 2).to_string());
        assert!(!last_deploy_rejected(
            tmp.path(),
            "my_fn",
            Some(&later),
            Some(true)
        ));
        assert!(last_deploy_rejected(
            tmp.path(),
            "my_fn",
            Some(&later),
            Some(false)
        ));

        clear_rejected_markers(tmp.path(), &["other".into(), "my_fn".into()]).unwrap();
        assert!(!rejected_marker_path(tmp.path(), "my_fn").exists());
        assert!(!last_deploy_rejected(
            tmp.path(),
            "my_fn",
            Some(&early),
            Some(true)
        ));
    }

    #[test]
    fn input_errors_map_to_local_error_variants() {
        let dir = Path::new("clickhouse/udfs/my_fn");
        let error = input_error(UdfInputError::Read {
            path: dir.join(DEFINITION_FILE),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        });
        assert!(
            matches!(&error, Error::UdfSourceInvalid { path, reason } if path == dir && reason == "has no udf.json")
        );
        assert!(matches!(
            input_error(UdfInputError::Read {
                path: dir.join(DEFINITION_FILE),
                source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            }),
            Error::Io(_)
        ));
        assert!(matches!(
            input_error(UdfInputError::Parse {
                path: dir.join(DEFINITION_FILE),
                source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
            }),
            Error::UdfDefinitionParse { .. }
        ));
        assert!(matches!(
            input_error(UdfInputError::Invalid {
                path: dir.to_path_buf(),
                reason: "is a symbolic link".into(),
            }),
            Error::UdfSourceInvalid { .. }
        ));
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    const UNKNOWN_TYPE: &str = "Code: 50. DB::Exception: Unknown data type family: Nope.";

    #[test]
    fn rejection_report_prefers_per_function_status_and_falls_back_to_unlisted_staged_names() {
        let staged = names(&["bad", "old", "rev"]);
        let loaded = names(&["old", "rev", "unmanaged"]);
        // 26.2+: the status query names the broken ones, staged ones only.
        let report = rejection_report(&staged, Some("bad\nhandmade\n"), UNKNOWN_TYPE, &loaded);
        assert!(report.per_function_status);
        assert_eq!(report.failed, names(&["bad"]));
        assert_eq!(report.failed_assuming_active, names(&["bad"]));

        // Older servers: staged but not listed means broken, once some
        // staged function shows the directory is read.
        let report = rejection_report(&staged, None, UNKNOWN_TYPE, &loaded);
        assert!(!report.per_function_status);
        assert!(report.directory_active);
        assert_eq!(report.failed, names(&["bad"]));

        // Nothing staged is listed: the directory may not be read yet.
        let report = rejection_report(&staged, None, UNKNOWN_TYPE, &names(&["unmanaged"]));
        assert!(!report.directory_active);
        assert!(report.failed.is_empty());
        assert_eq!(report.failed_assuming_active, staged);

        // A name clash in the error text names a function on any version.
        let clash = "Code: 609. DB::Exception: The function 'old' already exists.";
        let report = rejection_report(&staged, None, clash, &names(&["unmanaged"]));
        assert_eq!(report.failed, names(&["old"]));
    }

    fn probe(was_loaded: bool, overlay_fresh: bool) -> LoadProbe<'static> {
        LoadProbe {
            name: "rev",
            data_dir: Path::new("/nonexistent"),
            was_loaded,
            overlay_fresh,
        }
    }

    fn report(failed: &[&str], per_function_status: bool, active: bool) -> RejectionReport {
        RejectionReport {
            failed: names(failed),
            failed_assuming_active: names(failed),
            per_function_status,
            directory_active: active,
        }
    }

    #[test]
    fn a_function_that_loaded_despite_another_broken_one_is_deployed() {
        let loaded = names(&["bad_old", "rev"]);
        // 26.2+: loaded and not failed means the new definition loaded,
        // whether or not it was loaded before.
        for was_loaded in [false, true] {
            let outcome = rejected_outcome(
                &probe(was_loaded, false),
                &report(&["bad_old"], true, true),
                &loaded,
                UNKNOWN_TYPE.into(),
            );
            assert!(
                matches!(&outcome, Some(LoadOutcome::LoadedDespite { current, broken, details })
                    if current == &names(&["rev"]) && broken == &names(&["bad_old"])
                        && details == UNKNOWN_TYPE),
                "{was_loaded}"
            );
        }
        // Older servers: a function that was not loaded before must run
        // the new definition.
        let outcome = rejected_outcome(
            &probe(false, false),
            &report(&["bad"], false, true),
            &loaded,
            UNKNOWN_TYPE.into(),
        );
        assert!(
            matches!(&outcome, Some(LoadOutcome::LoadedDespite { current, .. })
            if current == &names(&["rev"]))
        );
    }

    #[test]
    fn a_broken_or_unconfirmed_function_is_rejected() {
        let loaded = names(&["rev"]);
        // Reported broken: blamed, even while an earlier definition is loaded.
        for per_function_status in [false, true] {
            let outcome = rejected_outcome(
                &probe(true, false),
                &report(&["bad", "rev"], per_function_status, true),
                &loaded,
                UNKNOWN_TYPE.into(),
            );
            assert!(
                matches!(&outcome, Some(LoadOutcome::Rejected { blocking, .. })
                if blocking == &names(&["bad", "rev"]))
            );
        }
        // Older servers cannot confirm a redeploy of a loaded function.
        let outcome = rejected_outcome(
            &probe(true, false),
            &report(&["bad"], false, true),
            &loaded,
            UNKNOWN_TYPE.into(),
        );
        assert!(
            matches!(&outcome, Some(LoadOutcome::Rejected { blocking, .. })
            if blocking == &names(&["bad"]))
        );
        // Not listed on an older server that reads the directory: broken.
        let unlisted = RejectionReport {
            failed: names(&["bad"]),
            failed_assuming_active: names(&["bad", "rev"]),
            per_function_status: false,
            directory_active: false,
        };
        let outcome = rejected_outcome(&probe(false, false), &unlisted, &[], UNKNOWN_TYPE.into());
        assert!(
            matches!(&outcome, Some(LoadOutcome::Rejected { blocking, .. })
            if blocking == &names(&["bad", "rev"]))
        );
    }

    #[test]
    fn an_unseen_function_is_pending_while_the_server_may_not_read_its_file() {
        // A freshly written overlay on an older server with no evidence yet.
        let unseen = report(&[], false, false);
        assert!(rejected_outcome(&probe(false, true), &unseen, &[], UNKNOWN_TYPE.into()).is_none());
        // 26.2+ lists every function file it read, so absence is pending.
        let unseen = report(&["bad"], true, true);
        assert!(
            rejected_outcome(&probe(false, false), &unseen, &[], UNKNOWN_TYPE.into()).is_none()
        );
    }

    #[test]
    fn reload_still_fails_warning_names_the_broken_udfs_and_the_way_out() {
        let one = reload_still_fails_warning("dev", &names(&["bad"]), UNKNOWN_TYPE);
        assert!(one.contains("UDF bad is broken"), "{one}");
        assert!(
            one.contains("`clickhousectl local udf remove bad --server dev`"),
            "{one}"
        );
        assert!(!one.contains("DB::Exception"), "{one}");
        let many = reload_still_fails_warning("dev", &names(&["a", "b"]), UNKNOWN_TYPE);
        assert!(many.contains("UDFs a, b are broken"), "{many}");
        let unknown = reload_still_fails_warning("dev", &[], UNKNOWN_TYPE);
        assert!(unknown.ends_with(UNKNOWN_TYPE), "{unknown}");
    }

    #[test]
    fn error_chain_appends_each_distinct_source() {
        #[derive(Debug)]
        struct Layer(&'static str, Option<Box<Layer>>);
        impl std::fmt::Display for Layer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
        impl std::error::Error for Layer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.1.as_deref().map(|layer| layer as _)
            }
        }
        let error = Layer(
            "error sending request for url (http://localhost:1/)",
            Some(Box::new(Layer(
                "client error (Connect)",
                Some(Box::new(Layer(
                    "Connection refused (os error 61)",
                    Some(Box::new(Layer("Connection refused (os error 61)", None))),
                ))),
            ))),
        );
        assert_eq!(
            error_chain(&error),
            "error sending request for url (http://localhost:1/): client error (Connect): \
             Connection refused (os error 61)"
        );
    }

    #[tokio::test]
    async fn an_unreachable_server_reports_why_in_details() {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let info: ServerInfo = serde_json::from_value(json!({
            "name": "dev",
            "pid": 1,
            "http_port": port,
            "tcp_port": port,
            "version": "26.9",
            "started_at": "0",
            "cwd": "/",
        }))
        .unwrap();
        match send_query(&info, "SELECT 1").await {
            Err(QueryError::Unreachable(details)) => {
                assert!(details.contains("error sending request"), "{details}");
                assert!(
                    details.to_ascii_lowercase().contains("connection refused"),
                    "{details}"
                );
            }
            _ => panic!("expected an unreachable server"),
        }
    }

    #[test]
    fn deploy_maps_a_missing_source_directory_by_variant() {
        let missing = input_error(UdfInputError::Missing {
            path: "clickhouse/udfs/my_fn".into(),
        });
        assert!(matches!(&missing, Error::UdfSourceInvalid { reason, .. }
            if reason == crate::udf::SOURCE_DIR_MISSING));
    }
}
