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
        Conditional::new("With --org", &[&op::ORGANIZATION_GET_LIST]),
    ])
    .unscoped()];

const CONTEXT_FILE: &str = "context.json";
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
    match lookup(load(dir)?, base_url) {
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
         `clickhousectl cloud use --org <NAME>`.",
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

fn load(dir: &Path) -> CloudResult<Option<Context>> {
    let path = context_path(dir);
    read(&path)?.map(|data| parse(&path, &data)).transpose()
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
    let mut context = load(dir)?.unwrap_or_default();
    context.api_url = Some(crate::cloud::auth::normalize_api_url(api_url));
    context.org = Some(org);
    write(dir, &context)
}

/// Remove the organization pin from `dir`, returning the one removed. The
/// file goes when nothing else is left in it, or when it cannot be parsed:
/// clearing is the remedy the parse error suggests.
fn clear_org(dir: &Path) -> CloudResult<Option<PinnedOrg>> {
    let path = context_path(dir);
    let Some(data) = read(&path)? else {
        return Ok(None);
    };
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
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else if let Some(org) = removed {
            println!("Unpinned organization {}", org.label());
        } else {
            println!("No organization was pinned");
        }
        return Ok(());
    }

    let context = load(&dir)?;
    let org = context.as_ref().and_then(|context| context.org.as_ref());
    let api_url = org.and(context.as_ref()).map(Context::api_base_url);
    if json {
        let output = UseOutput {
            org,
            api_url,
            path,
            cleared: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if let (Some(org), Some(api_url)) = (org, api_url) {
        println!("Organization: {}", org.label());
        println!("API: {api_url}");
        println!("Pinned in: {path}");
    } else {
        println!(
            "No organization pinned. Pin one with `clickhousectl cloud use --org <NAME>` \
             or `--org-id <ID>`."
        );
    }
    Ok(())
}

/// Resolve the `--org`/`--org-id` selector and pin it for this project.
pub(super) async fn run(client: &CloudClient, args: UseArgs, json: bool) -> CloudResult<()> {
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
        (None, None) => return run_local(&args, json),
    };

    let dir = crate::init::local_dir()?;
    save_org(&dir, client.base_url(), org.clone())?;
    let path = display_path(&context_path(&dir));
    if json {
        let output = UseOutput {
            org: Some(&org),
            api_url: Some(crate::cloud::auth::normalize_api_url(client.base_url())),
            path,
            cleared: None,
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
        assert!(load(&dir).unwrap().is_none());

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
