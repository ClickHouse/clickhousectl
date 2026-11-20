//! The project's pinned Cloud context, set by `cloud use` (#1509).
//!
//! `context.json` sits beside `credentials.json` in the project's
//! `.clickhouse/`, whose `.gitignore` keeps it out of version control, so a
//! pin belongs to one checkout, like git's `HEAD`. It is bound to the API it
//! was set against: a client resolved to another API ignores it.

use super::permissions::{Conditional, Declaration as Permission};
use crate::cloud::client::{CloudClient, CloudError, ResourceKind, Result as CloudResult, Scope};
use crate::failure::{ApiFailure, FailureKind};
use clap::Args;
use clickhouse_cloud_api::meta::operations as op;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// Showing or clearing the pin makes no call; pinning verifies the org first.
pub(super) const PERMISSIONS: &[Permission] = &[Permission::api("use", &[])
    .when(&[
        Conditional::new("With --org-id", &[&op::ORGANIZATION_GET]),
        Conditional::new("With --org-name", &[&op::ORGANIZATION_GET_LIST]),
    ])
    .unscoped()];

const CONTEXT_FILE: &str = "context.json";
/// The file format's version, not clickhousectl's: it changes only when a
/// later format cannot be read as this one, since added keys are carried
/// through by [`Context::other`].
const CONTEXT_VERSION: u32 = 1;

#[derive(Args)]
pub struct UseArgs {
    /// Remove the pinned organization
    #[arg(long, display_order = 1)]
    pub clear: bool,
}

/// The organization a project is pinned to, resolved and verified when it
/// was pinned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PinnedOrg {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PinnedOrg {
    fn label(&self) -> String {
        match &self.name {
            Some(name) => format!("{name} ({})", self.id),
            None => self.id.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Context {
    #[serde(default = "current_version")]
    version: u32,
    /// The normalized API base URL every pin in the file belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    api_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    org: Option<PinnedOrg>,
    /// Keys a later version writes (service, Postgres and branch pins,
    /// #1510), kept as they are across a write by this one.
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

fn current_version() -> u32 {
    CONTEXT_VERSION
}

impl Default for Context {
    fn default() -> Self {
        Self {
            version: CONTEXT_VERSION,
            api_url: None,
            org: None,
            other: serde_json::Map::new(),
        }
    }
}

impl Context {
    fn api_base_url(&self) -> String {
        self.api_url
            .as_deref()
            .map(crate::cloud::auth::normalize_api_url)
            .unwrap_or_else(|| crate::cloud::client::DEFAULT_BASE_URL.to_string())
    }
}

/// A pin that supplied this invocation's organization, kept so a later
/// failure can name it.
#[derive(Debug, Clone)]
pub(crate) struct ActivePin {
    pub org: PinnedOrg,
    path: PathBuf,
}

/// What the project's pin means for a client talking to one API.
#[derive(Debug, PartialEq, Eq)]
enum PinLookup {
    Unset,
    Applies(PinnedOrg),
    OtherApi { pinned_api: String },
}

fn lookup(context: Option<Context>, base_url: &str) -> PinLookup {
    let Some(context) = context else {
        return PinLookup::Unset;
    };
    let pinned_api = context.api_base_url();
    match context.org {
        None => PinLookup::Unset,
        Some(_) if pinned_api != crate::cloud::auth::normalize_api_url(base_url) => {
            PinLookup::OtherApi { pinned_api }
        }
        Some(org) => PinLookup::Applies(org),
    }
}

/// The organization pinned in `dir` when it belongs to `base_url`. Says on
/// stderr that the pin is used, or why it is ignored.
pub(crate) fn pinned_org_for(dir: &Path, base_url: &str) -> CloudResult<Option<ActivePin>> {
    let path = context_path(dir);
    let context = match load(dir)? {
        Stored::Missing => None,
        Stored::Newer { version } => {
            warn_newer(&path, version);
            return Ok(None);
        }
        Stored::Current(context) => Some(context),
    };
    match lookup(context, base_url) {
        PinLookup::Unset => Ok(None),
        PinLookup::OtherApi { pinned_api } => {
            eprintln!(
                "warning: ignoring the organization pinned in {}: it was pinned for {pinned_api}, \
                 but this command uses {}",
                display_path(&path),
                crate::cloud::auth::normalize_api_url(base_url),
            );
            Ok(None)
        }
        PinLookup::Applies(org) => {
            eprintln!(
                "Using organization {} pinned in {}",
                org.label(),
                display_path(&path)
            );
            Ok(Some(ActivePin { org, path }))
        }
    }
}

/// `error`, from a request scoped to the pinned organization that no longer
/// resolves, re-stated to name the pin and how to change it. The kind, the
/// failure classification and any structured detail are kept.
pub(crate) fn stale_pin_error(error: CloudError, pin: &ActivePin) -> CloudError {
    let message = format!(
        "{}\nOrganization {} comes from the pin in {}. Unpin it with \
         `clickhousectl cloud use --clear`, or pin another with \
         `clickhousectl cloud use --org-name <NAME>`.",
        error.message,
        pin.org.label(),
        display_path(&pin.path),
    );
    let details = error.details.map(|mut details| {
        details.message = message.clone();
        details
    });
    CloudError {
        message,
        details,
        ..error
    }
}

fn context_path(dir: &Path) -> PathBuf {
    dir.join(CONTEXT_FILE)
}

/// The path relative to the current directory when it is beneath it, as
/// people read it in a project; otherwise in full.
fn display_path(path: &Path) -> String {
    std::env::current_dir()
        .and_then(|cwd| cwd.canonicalize())
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
}

fn read(path: &Path) -> CloudResult<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(data) => Ok(Some(data)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(
            CloudError::new(format!("failed to read {}: {error}", path.display()))
                .with_failure(ApiFailure::new(FailureKind::Io)),
        ),
    }
}

fn parse(path: &Path, data: &str) -> CloudResult<Context> {
    serde_json::from_str(data).map_err(|error| {
        CloudError::new(format!(
            "failed to parse {}: {error}. Fix the file, or remove it with \
             `clickhousectl cloud use --clear`.",
            path.display()
        ))
        .with_failure(ApiFailure::new(FailureKind::Other))
    })
}

/// The format version `data` declares when it is newer than this one's,
/// read before the rest, so a newer file is recognized even when this
/// version cannot parse it. `None` when it is not JSON or declares no newer
/// version: such a file is this version's to parse, or to clear.
fn newer_version(data: &str) -> Option<u64> {
    #[derive(Deserialize)]
    struct Header {
        version: Option<serde_json::Value>,
    }
    let version = serde_json::from_str::<Header>(data)
        .ok()?
        .version?
        .as_u64()?;
    (version > u64::from(CONTEXT_VERSION)).then_some(version)
}

fn newer_reason(version: u64) -> String {
    format!(
        "written by a newer clickhousectl (format version {version}, this one understands \
         {CONTEXT_VERSION})"
    )
}

fn warn_newer(path: &Path, version: u64) {
    eprintln!(
        "warning: ignoring the pin in {}: it was {}; upgrade clickhousectl to use it",
        display_path(path),
        newer_reason(version)
    );
}

/// Pinning and clearing never rewrite or remove a newer file: it may hold
/// pins only the version that wrote it understands.
fn newer_error(path: &Path, version: u64) -> CloudError {
    CloudError::new(format!(
        "{} was {}; upgrade clickhousectl to change it",
        display_path(path),
        newer_reason(version)
    ))
    .with_failure(ApiFailure::new(FailureKind::Other))
}

/// What the project's `context.json` holds for this version.
#[derive(Debug)]
enum Stored {
    Missing,
    /// Written in a newer format than this version reads.
    Newer {
        version: u64,
    },
    Current(Context),
}

fn load(dir: &Path) -> CloudResult<Stored> {
    let path = context_path(dir);
    let Some(data) = read(&path)? else {
        return Ok(Stored::Missing);
    };
    if let Some(version) = newer_version(&data) {
        return Ok(Stored::Newer { version });
    }
    parse(&path, &data).map(Stored::Current)
}

fn write(dir: &Path, context: &Context) -> CloudResult<()> {
    crate::cloud::credentials::ensure_dir(dir)?;
    let json = serde_json::to_string_pretty(context)?;
    crate::paths::atomic_write(&context_path(dir), json.as_bytes(), 0o644)?;
    Ok(())
}

fn remove(path: &Path) -> CloudResult<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Pin `org` for `api_url` in `dir`, keeping any other keys in the file.
fn save_org(dir: &Path, api_url: &str, org: PinnedOrg) -> CloudResult<()> {
    let mut context = match load(dir)? {
        Stored::Missing => Context::default(),
        Stored::Newer { version } => return Err(newer_error(&context_path(dir), version)),
        Stored::Current(context) => context,
    };
    context.api_url = Some(crate::cloud::auth::normalize_api_url(api_url));
    context.org = Some(org);
    write(dir, &context)
}

/// Remove the organization pin from `dir`, returning the one removed. The
/// file goes when nothing else is left in it, or when it cannot be parsed:
/// clearing is the remedy the parse error suggests. A file in a newer format
/// is refused and left as it is.
fn clear_org(dir: &Path) -> CloudResult<Option<PinnedOrg>> {
    let path = context_path(dir);
    let Some(data) = read(&path)? else {
        return Ok(None);
    };
    if let Some(version) = newer_version(&data) {
        return Err(newer_error(&path, version));
    }
    let Ok(mut context) = parse(&path, &data) else {
        remove(&path)?;
        return Ok(None);
    };
    let removed = context.org.take();
    if context.other.is_empty() {
        remove(&path)?;
    } else if removed.is_some() {
        write(dir, &context)?;
    }
    Ok(removed)
}

#[derive(Serialize)]
struct UseOutput<'a> {
    org: Option<&'a PinnedOrg>,
    api_url: Option<String>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cleared: Option<bool>,
    /// The file's format version when a newer clickhousectl wrote it, so
    /// this one cannot use its pin.
    #[serde(skip_serializing_if = "Option::is_none")]
    newer_format_version: Option<u64>,
}

/// Show or clear the pin: neither needs credentials or the network.
pub(super) fn run_local(args: &UseArgs, json: bool) -> CloudResult<()> {
    let dir = crate::init::local_dir()?;
    let path = display_path(&context_path(&dir));
    if args.clear {
        let removed = clear_org(&dir)?;
        if json {
            let output = UseOutput {
                org: None,
                api_url: None,
                path,
                cleared: Some(removed.is_some()),
                newer_format_version: None,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else if let Some(org) = removed {
            println!("Unpinned organization {}", org.label());
        } else {
            println!("No organization was pinned");
        }
        return Ok(());
    }

    let (context, newer) = match load(&dir)? {
        Stored::Missing => (None, None),
        Stored::Newer { version } => (None, Some(version)),
        Stored::Current(context) => (Some(context), None),
    };
    let org = context.as_ref().and_then(|context| context.org.as_ref());
    let api_url = org.and(context.as_ref()).map(Context::api_base_url);
    if json {
        if let Some(version) = newer {
            warn_newer(&context_path(&dir), version);
        }
        let output = UseOutput {
            org,
            api_url,
            path,
            cleared: None,
            newer_format_version: newer,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if let Some(version) = newer {
        println!(
            "No usable organization pin: {path} was {}. Upgrade clickhousectl to use or \
             change it.",
            newer_reason(version)
        );
    } else if let (Some(org), Some(api_url)) = (org, api_url) {
        println!("Organization: {}", org.label());
        println!("API: {api_url}");
        println!("Pinned in: {path}");
    } else {
        println!(
            "No organization pinned. Pin one with `clickhousectl cloud use --org-name <NAME>` \
             or `--org-id <ID>`."
        );
    }
    Ok(())
}

/// The project's `.clickhouse/` to pin into. Never the home directory's,
/// which discovery selects from home itself: that one is global, so a pin
/// there would follow every directory without a project of its own.
fn pin_dir() -> CloudResult<PathBuf> {
    if crate::init::project_root_is_home()? {
        return Err(CloudError::usage(
            "the current directory is the home directory, whose .clickhouse/ is global, \
             not a project; run `clickhousectl cloud use` inside a project directory or set \
             CLICKHOUSECTL_PROJECT_DIR",
        )
        .with_failure(ApiFailure::new(FailureKind::Other)));
    }
    Ok(crate::init::local_dir()?)
}

/// Resolve the `--org-name`/`--org-id` selector and pin it for this project.
pub(super) async fn run(client: &CloudClient, args: UseArgs, json: bool) -> CloudResult<()> {
    if client.organization_selector() == (None, None) {
        return run_local(&args, json);
    }
    // Refused before any request: there is nowhere to keep the pin.
    let dir = pin_dir()?;
    let org = match client.organization_selector() {
        (Some(id), _) => {
            let organization = client.get_organization(id).await?;
            PinnedOrg {
                id: organization
                    .id
                    .map_or_else(|| id.to_string(), |id| id.to_string()),
                name: organization.name,
            }
        }
        (None, Some(name)) => {
            let rows = client.list_organizations().await?;
            // Names may collide; only --org-id can choose between them.
            let id = crate::cloud::shared::select_named_id_or(
                ResourceKind::Organization,
                Scope::Account,
                name,
                rows.iter()
                    .map(|row| (row.name.as_deref(), row.id.as_ref())),
                |ids| format!("choose one with --org-id <ID>: {}", ids.join(", ")),
            )?;
            PinnedOrg {
                id,
                name: Some(name.to_string()),
            }
        }
        (None, None) => unreachable!("an empty selector shows the pin"),
    };

    save_org(&dir, client.base_url(), org.clone())?;
    let path = display_path(&context_path(&dir));
    if json {
        let output = UseOutput {
            org: Some(&org),
            api_url: Some(crate::cloud::auth::normalize_api_url(client.base_url())),
            path,
            cleared: None,
            newer_format_version: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "Pinned organization {} for this project ({path})",
            org.label()
        );
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn org(id: &str) -> PinnedOrg {
        PinnedOrg {
            id: id.into(),
            name: Some("acme".into()),
        }
    }

    /// Pin `id` for `api_url` in `dir`, as `cloud use` would.
    pub(crate) fn pin(dir: &Path, api_url: &str, id: &str) {
        save_org(dir, api_url, org(id)).unwrap();
    }

    #[test]
    fn pinning_creates_a_gitignored_file_and_clearing_removes_it_idempotently() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".clickhouse");
        assert!(matches!(load(&dir).unwrap(), Stored::Missing));

        pin(&dir, "https://api.clickhouse.cloud", "org-1");
        assert_eq!(
            std::fs::read_to_string(dir.join(".gitignore")).unwrap(),
            "*\n"
        );
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(context_path(&dir)).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({
                "version": 1,
                "api_url": "https://api.clickhouse.cloud/v1",
                "org": {"id": "org-1", "name": "acme"},
            })
        );

        assert_eq!(clear_org(&dir).unwrap(), Some(org("org-1")));
        assert!(!context_path(&dir).exists());
        assert_eq!(clear_org(&dir).unwrap(), None);
    }

    #[test]
    fn keys_a_later_version_writes_survive_pinning_and_clearing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let service = serde_json::json!({"id": "svc-1", "name": "demo"});
        std::fs::write(
            context_path(dir),
            serde_json::json!({"version": 1, "service": service}).to_string(),
        )
        .unwrap();

        pin(dir, "https://api.clickhouse.cloud", "org-1");
        assert_eq!(clear_org(dir).unwrap(), Some(org("org-1")));
        let left: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(context_path(dir)).unwrap()).unwrap();
        assert_eq!(left["service"], service);
        assert!(left.get("org").is_none());
    }

    #[test]
    fn an_unparsable_file_is_an_error_naming_it_and_clearing_removes_it() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(context_path(dir), "{not json").unwrap();
        let error = load(dir).unwrap_err();
        assert!(
            error
                .message
                .contains(&context_path(dir).display().to_string())
        );
        assert!(error.message.contains("cloud use --clear"));
        assert_eq!(clear_org(dir).unwrap(), None);
        assert!(!context_path(dir).exists());
    }

    /// A file from a newer clickhousectl, in a shape this version cannot
    /// parse.
    const NEWER: &str = r#"{"version": 2, "org": ["a", "shape", "v1", "cannot", "read"]}"#;

    #[test]
    fn a_newer_format_is_ignored_on_lookup_without_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(context_path(dir), NEWER).unwrap();
        assert!(matches!(load(dir).unwrap(), Stored::Newer { version: 2 }));
        assert!(
            pinned_org_for(dir, "https://api.clickhouse.cloud")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn pinning_and_clearing_refuse_a_newer_format_and_leave_it_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(context_path(dir), NEWER).unwrap();
        for error in [
            save_org(dir, "https://api.clickhouse.cloud", org("org-1")).unwrap_err(),
            clear_org(dir).unwrap_err(),
        ] {
            assert!(
                error.message.contains("newer clickhousectl"),
                "{}",
                error.message
            );
            assert!(
                error.message.contains("format version 2"),
                "{}",
                error.message
            );
            assert!(
                error.message.contains("upgrade clickhousectl"),
                "{}",
                error.message
            );
        }
        assert_eq!(std::fs::read_to_string(context_path(dir)).unwrap(), NEWER);
    }

    #[test]
    fn a_broken_file_of_this_format_or_no_readable_version_is_still_cleared() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        for broken in [
            r#"{"version": 1, "org": "not an object"}"#,
            r#"{"org": "not an object"}"#,
            r#"{"version": 2, "org""#,
        ] {
            std::fs::write(context_path(dir), broken).unwrap();
            assert!(load(dir).is_err(), "{broken}");
            assert_eq!(clear_org(dir).unwrap(), None, "{broken}");
            assert!(!context_path(dir).exists(), "{broken}");
        }
    }

    #[test]
    fn a_pin_applies_only_to_the_api_it_was_set_against() {
        let context = |api_url: Option<&str>| Context {
            api_url: api_url.map(str::to_string),
            org: Some(org("org-1")),
            ..Context::default()
        };
        // Spellings of the same API, and a file without api_url, are production.
        for (pinned, client) in [
            (
                Some("https://api.clickhouse.cloud/v1"),
                "https://api.clickhouse.cloud",
            ),
            (None, "https://api.clickhouse.cloud/v1/"),
        ] {
            assert_eq!(
                lookup(Some(context(pinned)), client),
                PinLookup::Applies(org("org-1"))
            );
        }
        assert_eq!(
            lookup(
                Some(context(None)),
                "https://api.control-plane.clickhouse-dev.com"
            ),
            PinLookup::OtherApi {
                pinned_api: "https://api.clickhouse.cloud/v1".into()
            }
        );
        assert_eq!(
            lookup(None, "https://api.clickhouse.cloud"),
            PinLookup::Unset
        );
        assert_eq!(
            lookup(Some(Context::default()), "https://api.clickhouse.cloud"),
            PinLookup::Unset
        );
    }
}
