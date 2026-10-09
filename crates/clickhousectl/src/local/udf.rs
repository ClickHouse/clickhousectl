//! `local udf`: executable UDFs for project-local ClickHouse servers.
//!
//! A UDF lives in a directory (`clickhouse/udfs/<name>/`) holding
//! `udf.json`, the same definition `cloud udf` accepts, next to its files.
//! `deploy` renders that definition into ClickHouse's `<function>` XML under
//! the server's data directory, copies the sources next to it, and points the
//! server at both through a managed `config.d` overlay. The server picks new
//! files up on its own; when it is running we also ask it to reload at once.

use crate::error::{Error, Result};
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
const DEFAULT_FORMAT: &str = "TabSeparated";
const PYTHON_CANDIDATES: [&str; 2] = ["python3.11", "python3"];
/// Copied but not installed: a local server runs the interpreter as is.
const REQUIREMENTS_FILE: &str = "requirements.txt";
const REQUIREMENTS_NOTICE: &str = "requirements.txt is not installed locally; install its packages \
     into the interpreter you deploy with (e.g. a virtualenv passed with --python).";
const NATIVE_UNSUPPORTED: &str = "native UDFs are Linux binaries and cannot run on this host's \
     local server; deploy them on Linux or use runtime python3.11";
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// How long `deploy` keeps asking a running server to reload before giving
/// up on the function. Covers the server's own config-reload period.
const LOAD_POLL_TIMEOUT: Duration = Duration::from_secs(10);
const LOAD_POLL_INTERVAL: Duration = Duration::from_millis(250);
const RELOAD_FUNCTIONS_SQL: &str = "SYSTEM RELOAD FUNCTIONS";
const LOADED_FUNCTIONS_SQL: &str = "SELECT name FROM system.functions \
     WHERE origin = 'ExecutableUserDefined' ORDER BY name FORMAT TabSeparated";

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
    /// Cloud-only fields present in the definition, in the API's spelling.
    fn ignored_fields(&self) -> Vec<String> {
        [
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
/// Cloud-only fields it carries.
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

    let out = UdfInitOutput {
        name: name.to_owned(),
        dir: target.display().to_string(),
        created,
        next_step,
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
    let dir = udf::resolve_source_dir(parent, name).map_err(input_error)?;
    let (definition, ignored_fields) = load_local_definition(&dir, name)?;
    let runtime = definition.runtime.kind();
    let native_arch = match runtime {
        UdfRuntimeKind::Native => Some(native_host_arch()?),
        UdfRuntimeKind::Python311 => None,
    };
    let entries = udf::collect_source_entries(&dir, runtime).map_err(input_error)?;
    let ignored_files = ignored_files(&entries, runtime);

    let (target, lock) = server_target(server_name)?;
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

    let scripts_dir = target.data_dir.join(SCRIPTS_DIR).join(name);
    let executables: Vec<PathBuf> = match native_arch {
        Some(_) => NATIVE_ARCH_DIRS
            .iter()
            .map(|arch| Path::new(arch).join(NATIVE_ENTRYPOINT))
            .collect(),
        None => Vec::new(),
    };
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
    write_overlay(&target.data_dir)?;
    drop(lock);

    let loaded = match &target.running {
        Some(info) => {
            let deploy_command = deploy_command(name, parent, server_name);
            // Every function file reloads together, so a rejected definition
            // also blocks every later reload on this server until it is fixed.
            match send_query(info, RELOAD_FUNCTIONS_SQL).await {
                Ok(_) => {}
                Err(QueryError::Rejected(details)) => {
                    return Err(Error::UdfRejected {
                        name: name.to_owned(),
                        server: server_name.to_owned(),
                        source_dir: dir.clone(),
                        deploy_command,
                        details,
                    });
                }
                Err(error) => return Err(error.into_error(server_name)),
            }
            if !wait_until_loaded(info, server_name, name).await? {
                return Err(Error::UdfNotLoaded {
                    name: name.to_owned(),
                    server: server_name.to_owned(),
                    log_path: std::path::absolute(server::server_log_path(server_name))?,
                    deploy_command,
                });
            }
            Some(true)
        }
        None => None,
    };

    if !json && !ignored_files.is_empty() {
        // Not `eprintln!`, which panics on a closed stderr.
        let _ = writeln!(std::io::stderr(), "{REQUIREMENTS_NOTICE}");
    }
    let out = UdfDeployOutput {
        name: name.to_owned(),
        server: server_name.to_owned(),
        r#type: definition.kind.as_str().to_owned(),
        runtime: runtime.as_str().to_owned(),
        server_running: target.running.is_some(),
        reloaded: target.running.is_some(),
        loaded,
        interpreter: interpreter.map(|path| path.display().to_string()),
        ignored_fields,
        ignored_files,
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

/// Files copied with the sources that the local server does nothing with.
fn ignored_files(entries: &[SourceEntry], runtime: UdfRuntimeKind) -> Vec<String> {
    let requirements = runtime == UdfRuntimeKind::Python311
        && entries
            .iter()
            .any(|entry| !entry.is_dir && entry.relative == Path::new(REQUIREMENTS_FILE));
    if requirements {
        vec![REQUIREMENTS_FILE.to_owned()]
    } else {
        Vec::new()
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
            "native UDFs ship amd64 and arm64 binaries, but this host's CPU is {other}; \
             use runtime python3.11"
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
    let command = format!("{} {}", shell_quote(&interpreter)?, shell_quote(&script)?);
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

fn absolute(path: PathBuf) -> Result<PathBuf> {
    Ok(std::path::absolute(path)?)
}

/// Single-quote a path for `sh -c`, which ClickHouse uses when
/// `execute_direct` is off. Single quotes inside the path cannot be carried.
fn shell_quote(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .filter(|text| !text.contains('\''))
        .ok_or_else(|| Error::UdfSourceInvalid {
            path: path.to_path_buf(),
            reason: "contains a single quote or invalid UTF-8, which the server's shell \
                     command cannot carry; move the project or interpreter"
                .into(),
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
         <!-- Managed by clickhousectl: rewritten on every server start and udf deploy/remove. -->\n    \
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

fn function_xml_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir
        .join(FUNCTIONS_DIR)
        .join(format!("{name}{FUNCTION_FILE_SUFFIX}"))
}

fn sidecar_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(FUNCTIONS_DIR).join(format!("{name}.json"))
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
        return Err(Error::UdfServerNotFound(name.to_owned()));
    }
    Ok((ServerTarget { data_dir, running }, lock))
}

fn require_running<'a>(target: &'a ServerTarget, name: &str) -> Result<&'a ServerInfo> {
    target
        .running
        .as_ref()
        .ok_or_else(|| Error::ServerNotRunning(name.to_owned()))
}

/// Why a statement failed: the server could not be reached, or it answered
/// with an error. Deploy tells a rejected definition apart from the rest.
enum QueryError {
    Client(reqwest::Error),
    Unreachable(String),
    Rejected(String),
}

impl QueryError {
    fn into_error(self, server: &str) -> Error {
        match self {
            Self::Client(error) => error.into(),
            Self::Unreachable(details) | Self::Rejected(details) => Error::UdfQueryFailed {
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
    let unreachable = |error: reqwest::Error| QueryError::Unreachable(error.to_string());
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

async fn http_query(info: &ServerInfo, server: &str, sql: &str) -> Result<String> {
    send_query(info, sql)
        .await
        .map_err(|error| error.into_error(server))
}

async fn reload_functions(info: &ServerInfo, server: &str) -> Result<()> {
    http_query(info, server, RELOAD_FUNCTIONS_SQL)
        .await
        .map(drop)
}

async fn loaded_function_names(info: &ServerInfo, server: &str) -> Result<Vec<String>> {
    let body = http_query(info, server, LOADED_FUNCTIONS_SQL).await?;
    Ok(body
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

/// After a reload, give the server time to notice a freshly written overlay
/// (its config reloader runs periodically) before concluding the function
/// did not load.
async fn wait_until_loaded(info: &ServerInfo, server: &str, name: &str) -> Result<bool> {
    let deadline = Instant::now() + LOAD_POLL_TIMEOUT;
    loop {
        if loaded_function_names(info, server)
            .await?
            .iter()
            .any(|loaded| loaded == name)
        {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        tokio::time::sleep(LOAD_POLL_INTERVAL).await;
        reload_functions(info, server).await?;
    }
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
        .map(|staged| UdfListEntry {
            loaded: loaded.as_ref().map(|loaded| loaded.contains(&staged.name)),
            name: staged.name,
            r#type: staged.kind,
            runtime: staged.runtime,
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
            reload_functions(info, server_name).await?;
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
    reload_functions(info, server_name).await?;
    let loaded = loaded_function_names(info, server_name).await?;
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
        let entries = vec![entry("main.py"), entry("requirements.txt")];
        assert_eq!(
            ignored_files(&entries, UdfRuntimeKind::Python311),
            vec!["requirements.txt"]
        );
        assert!(ignored_files(&entries[..1], UdfRuntimeKind::Python311).is_empty());
        assert!(ignored_files(&entries, UdfRuntimeKind::Native).is_empty());
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
            shell_quote(Path::new("/opt/my python/bin/python3")).unwrap(),
            "'/opt/my python/bin/python3'"
        );
        assert!(matches!(
            shell_quote(Path::new("/opt/it's/python3")).unwrap_err(),
            Error::UdfSourceInvalid { .. }
        ));
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
}
