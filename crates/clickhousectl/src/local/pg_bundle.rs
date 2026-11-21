//! Portable PostgreSQL bundles for the native `local postgres` runtime.
//!
//! A bundle is a prebuilt PostgreSQL plus pg_clickhouse for one target,
//! published as a tarball next to a `manifest.json` that names its sha256.
//! This module resolves a requested version to a bundle, downloads and
//! verifies it, and keeps extracted bundles in `~/.clickhouse/postgres/`.
//! Running them is `local::pg_native`'s job.

use crate::error::{Error, NetworkStage, Result};
use crate::paths;
use crate::version_manager::network;
use crate::version_manager::platform::{Arch, Os, Platform};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Replaces the bundle source: an `https://` base URL or a `file:///abs/dir`
/// holding `manifest.json` and the tarballs (see `docs/development.md`).
pub(crate) const BUNDLES_URL_ENV: &str = "CLICKHOUSECTL_POSTGRES_BUNDLES_URL";
/// Where `manifest.json` and the tarballs it names are published. No request
/// to it, or to anything it redirects to, carries credentials.
const DEFAULT_BUNDLES_URL: &str = "https://github.com/sdairs/pg-bundles/releases/latest/download";
const MANIFEST_FILE: &str = "manifest.json";
const SUPPORTED_SCHEMA: u64 = 1;
const ARCHIVE_SUFFIX: &str = ".tar.gz";
/// Written into an extracted bundle before it is renamed into the cache, so a
/// directory that has it is complete.
const MARKER_FILE: &str = "clickhousectl-bundle.json";
const KEY_SHA_PREFIX_LEN: usize = 12;
const STAGING_PREFIX: &str = ".staging-";

/// Majors the native runtime and the Docker image tags both accept.
pub(crate) const SUPPORTED_MAJORS: [&str; 2] = ["17", "18"];

/// One bundle as the manifest describes it. Also the cache marker's content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BundleSpec {
    pub postgres_version: String,
    pub postgres_major: String,
    #[serde(default)]
    pub extensions: BTreeMap<String, String>,
    pub target: String,
    pub file: String,
    pub sha256: String,
}

impl BundleSpec {
    /// The bundle's name: its file name without the archive suffix, which is
    /// also the tarball's single top-level directory.
    pub(crate) fn name(&self) -> &str {
        self.file.strip_suffix(ARCHIVE_SUFFIX).unwrap_or(&self.file)
    }

    /// The cache directory name. The sha256 prefix keeps a rebuilt bundle
    /// published under the same name apart from the one already extracted.
    fn cache_key(&self) -> String {
        format!("{}-{}", self.name(), &self.sha256[..KEY_SHA_PREFIX_LEN])
    }

    fn version_key(&self) -> Vec<u64> {
        version_key(&self.postgres_version)
    }

    /// Reject a manifest entry this CLI would not install safely.
    fn validate(&self) -> std::result::Result<(), String> {
        let name = self
            .file
            .strip_suffix(ARCHIVE_SUFFIX)
            .ok_or_else(|| format!("bundle file '{}' is not a {ARCHIVE_SUFFIX}", self.file))?;
        let name_ok = !name.is_empty()
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        if !name_ok {
            return Err(format!(
                "bundle file '{}' is not a plain file name",
                self.file
            ));
        }
        if self.sha256.len() != 64 || !self.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "bundle '{}' has no valid sha256 in the manifest",
                self.file
            ));
        }
        Ok(())
    }
}

fn version_key(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// A Postgres version the native runtime can serve: a major, or `major.minor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PgVersionRequest {
    pub major: String,
    /// The full `major.minor` when the user pinned one.
    pub exact: Option<String>,
}

impl PgVersionRequest {
    pub(crate) fn major(major: &str) -> Self {
        Self {
            major: major.to_string(),
            exact: None,
        }
    }

    /// `17`, `18` or `<major>.<minor>`. `None` for anything else, such as a
    /// Docker image tag with a variant (`18-bookworm`).
    pub(crate) fn parse(tag: &str) -> Option<Self> {
        let (major, minor) = match tag.split_once('.') {
            Some((major, minor)) => (major, Some(minor)),
            None => (tag, None),
        };
        if !SUPPORTED_MAJORS.contains(&major) {
            return None;
        }
        match minor {
            None => Some(Self::major(major)),
            Some(minor) if !minor.is_empty() && minor.chars().all(|c| c.is_ascii_digit()) => {
                Some(Self {
                    major: major.to_string(),
                    exact: Some(tag.to_string()),
                })
            }
            Some(_) => None,
        }
    }

    fn matches(&self, spec: &BundleSpec) -> bool {
        spec.postgres_major == self.major
            && self
                .exact
                .as_deref()
                .is_none_or(|exact| version_key(exact) == spec.version_key())
    }

    fn label(&self) -> &str {
        self.exact.as_deref().unwrap_or(&self.major)
    }
}

/// An extracted bundle in the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledBundle {
    pub dir: PathBuf,
    pub spec: BundleSpec,
}

impl InstalledBundle {
    pub(crate) fn bin(&self, program: &str) -> PathBuf {
        self.dir.join("bin").join(program)
    }
}

/// The bundle target for a platform. Linux is always musl: the bundle ships
/// its own loader, so one build serves every distribution.
pub(crate) fn target_triple(platform: Platform) -> &'static str {
    match (platform.os, platform.arch) {
        (Os::MacOS, Arch::Aarch64) => "aarch64-apple-darwin",
        (Os::MacOS, Arch::X86_64) => "x86_64-apple-darwin",
        (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux-musl",
        (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux-musl",
    }
}

/// Where extracted bundles are kept.
pub(crate) fn cache_root() -> Result<PathBuf> {
    Ok(paths::base_dir()?.join("postgres"))
}

/// Parse a manifest and return its bundles. An unknown `schema` is rejected
/// before anything else is read, since its fields may mean something else.
fn parse_manifest(bytes: &[u8]) -> Result<Vec<BundleSpec>> {
    #[derive(Deserialize)]
    struct Schema {
        schema: u64,
    }
    #[derive(Deserialize)]
    struct Manifest {
        #[serde(default)]
        bundles: Vec<BundleSpec>,
    }
    let invalid = || Error::PostgresBundleUnavailable("the bundle manifest is not valid".into());
    let schema: Schema = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if schema.schema != SUPPORTED_SCHEMA {
        return Err(Error::PostgresBundleUnavailable(format!(
            "the bundle manifest has schema {}, and this clickhousectl reads schema \
             {SUPPORTED_SCHEMA}; run `clickhousectl update`",
            schema.schema
        )));
    }
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    Ok(manifest.bundles)
}

/// The newest bundle for `target` that satisfies `request`.
fn select_newest<'a>(
    bundles: impl IntoIterator<Item = &'a BundleSpec>,
    target: &str,
    request: &PgVersionRequest,
) -> Option<&'a BundleSpec> {
    bundles
        .into_iter()
        .filter(|spec| spec.target == target && request.matches(spec))
        .max_by_key(|spec| spec.version_key())
}

/// Every complete bundle in `root`. Unreadable or foreign entries are skipped:
/// the cache is a cache, and a start can always install again.
fn installed_in(root: &Path) -> Vec<InstalledBundle> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            let dir = entry.ok()?.path();
            if dir.file_name()?.to_str()?.starts_with('.') {
                return None;
            }
            let marker = std::fs::read(dir.join(MARKER_FILE)).ok()?;
            let spec: BundleSpec = serde_json::from_slice(&marker).ok()?;
            Some(InstalledBundle { dir, spec })
        })
        .collect()
}

fn newest_installed_in(
    root: &Path,
    target: &str,
    request: &PgVersionRequest,
) -> Option<InstalledBundle> {
    let installed = installed_in(root);
    let newest = select_newest(installed.iter().map(|b| &b.spec), target, request)?.clone();
    installed.into_iter().find(|bundle| bundle.spec == newest)
}

/// The newest cached bundle for `request` on this machine. Never uses the
/// network, so resuming an instance works offline.
pub(crate) fn newest_installed(request: &PgVersionRequest) -> Result<Option<InstalledBundle>> {
    let target = target_triple(Platform::detect()?);
    Ok(newest_installed_in(&cache_root()?, target, request))
}

/// Where bundles come from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BundleSource {
    Https(url::Url),
    Directory(PathBuf),
}

impl BundleSource {
    fn from_env() -> Result<Self> {
        Self::parse(std::env::var(BUNDLES_URL_ENV).ok().as_deref())
    }

    fn parse(override_url: Option<&str>) -> Result<Self> {
        let raw = override_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_BUNDLES_URL);
        let invalid = || {
            Error::PostgresBundleUnavailable(format!(
                "{BUNDLES_URL_ENV} must be an https:// URL or a file:///absolute/directory"
            ))
        };
        // A trailing slash makes `join` resolve files inside the base rather
        // than beside its last segment.
        let base = if raw.ends_with('/') {
            raw.to_string()
        } else {
            format!("{raw}/")
        };
        let url = url::Url::parse(&base).map_err(|_| invalid())?;
        match url.scheme() {
            "https" => Ok(Self::Https(url)),
            "file" => url
                .to_file_path()
                .map(Self::Directory)
                .map_err(|_| invalid()),
            _ => Err(invalid()),
        }
    }

    async fn manifest(&self) -> Result<Vec<u8>> {
        match self {
            Self::Directory(dir) => Ok(std::fs::read(dir.join(MANIFEST_FILE))?),
            Self::Https(base) => {
                let url = base
                    .join(MANIFEST_FILE)
                    .expect("a file name joins onto a base URL")
                    .to_string();
                let stage = NetworkStage::Download;
                let client = network::client(network::METADATA_POLICY, stage, &url)?;
                let fetch = async {
                    let response = network::send(client.get(&url), stage, &url).await?;
                    let response = network::ensure_success(response, stage, &url)?;
                    response
                        .bytes()
                        .await
                        .map(|bytes| bytes.to_vec())
                        .map_err(|error| network::failure_from_reqwest(stage, &url, &error))
                };
                Ok(network::with_operation_timeout(
                    network::RESOLUTION_OPERATION_TIMEOUT,
                    stage,
                    &url,
                    fetch,
                )
                .await??)
            }
        }
    }

    /// Fetch `spec`'s tarball to `dest`. `spec.file` was validated as a plain
    /// file name, so it cannot point outside the source.
    async fn fetch(&self, spec: &BundleSpec, dest: &Path, json: bool) -> Result<()> {
        match self {
            Self::Directory(dir) => {
                std::fs::copy(dir.join(&spec.file), dest)?;
                Ok(())
            }
            Self::Https(base) => {
                let url = base
                    .join(&spec.file)
                    .expect("a file name joins onto a base URL");
                crate::version_manager::download::download_file(url.as_str(), dest, json).await
            }
        }
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Check a downloaded tarball against the manifest's sha256 before anything
/// reads its contents.
fn verify_sha256(archive: &Path, spec: &BundleSpec) -> Result<()> {
    let actual = sha256_file(archive)?;
    if actual.eq_ignore_ascii_case(&spec.sha256) {
        Ok(())
    } else {
        Err(Error::PostgresBundleChecksum {
            file: spec.file.clone(),
            expected: spec.sha256.to_ascii_lowercase(),
            actual,
        })
    }
}

/// A staging directory under the cache root, removed when dropped.
struct Staging(PathBuf);

impl Staging {
    fn create(root: &Path) -> Result<Self> {
        paths::ensure_dir(root)?;
        let path = root.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Unpack a verified tarball into `staging` and return its bundle directory.
fn extract(archive: &Path, staging: &Path, spec: &BundleSpec) -> Result<PathBuf> {
    let extraction_error = |source| Error::ExtractArchive {
        archive: archive.to_path_buf(),
        destination: staging.to_path_buf(),
        source,
    };
    let file = std::fs::File::open(archive).map_err(extraction_error)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    // `unpack` refuses entries that would land outside `staging`.
    tar.unpack(staging).map_err(extraction_error)?;
    // tar stops at its end-of-archive blocks; reading on validates the gzip
    // trailer.
    std::io::copy(&mut tar.into_inner(), &mut std::io::sink()).map_err(extraction_error)?;

    let dir = staging.join(spec.name());
    if !dir.join("bin").join("postgres").is_file() {
        return Err(Error::Extract(format!(
            "bundle '{}' does not contain {}/bin/postgres",
            spec.file,
            spec.name()
        )));
    }
    Ok(dir)
}

/// Download, verify and extract `spec` into `root`, returning the cached
/// bundle. The extracted directory is renamed into place in one step, so a
/// concurrent installer of the same bundle either wins or is used.
async fn install_into(
    root: &Path,
    source: &BundleSource,
    spec: &BundleSpec,
    force: bool,
    json: bool,
) -> Result<InstalledBundle> {
    let staging = Staging::create(root)?;
    let archive = staging.0.join(&spec.file);
    if !json {
        eprintln!(
            "Downloading PostgreSQL {} for {}...",
            spec.postgres_version, spec.target
        );
    }
    source.fetch(spec, &archive, json).await?;
    verify_sha256(&archive, spec)?;

    let unpack_dir = staging.0.join("unpack");
    std::fs::create_dir(&unpack_dir)?;
    let extracted = extract(&archive, &unpack_dir, spec)?;
    std::fs::write(
        extracted.join(MARKER_FILE),
        serde_json::to_vec_pretty(spec)?,
    )?;

    let dir = root.join(spec.cache_key());
    if force && dir.exists() {
        // Move the old copy aside first: a running postmaster keeps its open
        // files, and the staging guard deletes the directory afterwards.
        std::fs::rename(&dir, staging.0.join("replaced"))?;
    }
    match std::fs::rename(&extracted, &dir) {
        Ok(()) => {}
        // Another installer finished first; its copy passed the same check.
        Err(_) if dir.join(MARKER_FILE).is_file() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(InstalledBundle {
        dir,
        spec: spec.clone(),
    })
}

/// Make the newest bundle for `request` available and return it.
///
/// Reads the manifest, then uses the cached copy of the bundle it names or
/// installs it. When the manifest cannot be fetched, a cached bundle for the
/// request is used instead, so a start works offline. `force` installs again
/// even when cached, and then needs the manifest.
pub(crate) async fn ensure_bundle(
    request: &PgVersionRequest,
    force: bool,
    json: bool,
) -> Result<InstalledBundle> {
    let target = target_triple(Platform::detect()?);
    let root = cache_root()?;
    let source = BundleSource::from_env()?;
    ensure_bundle_in(&root, &source, target, request, force, json).await
}

async fn ensure_bundle_in(
    root: &Path,
    source: &BundleSource,
    target: &str,
    request: &PgVersionRequest,
    force: bool,
    json: bool,
) -> Result<InstalledBundle> {
    let manifest = match source.manifest().await {
        Ok(bytes) => bytes,
        Err(error) => {
            let cached = (!force)
                .then(|| newest_installed_in(root, target, request))
                .flatten();
            let Some(cached) = cached else {
                return Err(error);
            };
            if !json {
                eprintln!(
                    "Note: could not check for a newer PostgreSQL bundle; using installed {}",
                    cached.spec.postgres_version
                );
            }
            return Ok(cached);
        }
    };
    let bundles = parse_manifest(&manifest)?;
    let spec = select_newest(&bundles, target, request).ok_or_else(|| {
        Error::PostgresBundleUnavailable(format!(
            "no PostgreSQL {} bundle is published for {target}; use `--docker` to run a \
             Postgres image instead",
            request.label()
        ))
    })?;
    spec.validate().map_err(Error::PostgresBundleUnavailable)?;

    if !force
        && let Some(cached) = installed_in(root)
            .into_iter()
            .find(|bundle| bundle.spec == *spec)
    {
        return Ok(cached);
    }
    install_into(root, source, spec, force, json).await
}

/// The bundle an existing instance of `major` starts with: the newest cached
/// one, since data directories are compatible across minors of a major. Only
/// an empty cache reaches the network.
pub(crate) async fn bundle_for_existing(major: &str, json: bool) -> Result<InstalledBundle> {
    let request = PgVersionRequest::major(major);
    match newest_installed(&request)? {
        Some(bundle) => Ok(bundle),
        None => ensure_bundle(&request, false, json).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn spec(version: &str, target: &str, sha256: &str) -> BundleSpec {
        BundleSpec {
            postgres_version: version.to_string(),
            postgres_major: version.split('.').next().unwrap().to_string(),
            extensions: BTreeMap::from([("pg_clickhouse".to_string(), "0.11.1".to_string())]),
            target: target.to_string(),
            file: format!("pg-{version}-pgch-0.11.1-{target}.tar.gz"),
            sha256: sha256.to_string(),
        }
    }

    fn manifest(schema: u64, bundles: &[BundleSpec]) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": schema,
            "release": "v0.3.0",
            "built_at": "2026-11-16T00:00:00Z",
            "source": { "repo": "sdairs/pg-bundles", "commit": "abc" },
            "bundles": bundles,
        }))
        .unwrap()
    }

    #[test]
    fn every_supported_platform_maps_to_its_bundle_target() {
        for (os, arch, target) in [
            (Os::MacOS, Arch::Aarch64, "aarch64-apple-darwin"),
            (Os::MacOS, Arch::X86_64, "x86_64-apple-darwin"),
            (Os::Linux, Arch::Aarch64, "aarch64-unknown-linux-musl"),
            (Os::Linux, Arch::X86_64, "x86_64-unknown-linux-musl"),
        ] {
            assert_eq!(target_triple(Platform { os, arch }), target);
        }
    }

    #[test]
    fn native_versions_are_a_supported_major_with_an_optional_numeric_minor() {
        assert_eq!(
            PgVersionRequest::parse("18"),
            Some(PgVersionRequest::major("18"))
        );
        let pinned = PgVersionRequest::parse("17.11").unwrap();
        assert_eq!(pinned.major, "17");
        assert_eq!(pinned.exact.as_deref(), Some("17.11"));
        for tag in ["16", "19", "18-bookworm", "18.1-alpine", "18.", "18.x", ""] {
            assert_eq!(PgVersionRequest::parse(tag), None, "{tag}");
        }
    }

    #[test]
    fn manifest_parsing_reads_extra_fields_and_rejects_an_unknown_schema() {
        let bundles = [spec("18.7", "aarch64-apple-darwin", SHA_A)];
        let mut value: serde_json::Value = serde_json::from_slice(&manifest(1, &bundles)).unwrap();
        value["bundles"][0]["size"] = 11370169.into();
        value["bundles"][0]["future_field"] = true.into();
        assert_eq!(
            parse_manifest(&serde_json::to_vec(&value).unwrap()).unwrap(),
            bundles
        );

        for bytes in [manifest(2, &bundles), b"{}".to_vec(), b"not json".to_vec()] {
            assert!(matches!(
                parse_manifest(&bytes),
                Err(Error::PostgresBundleUnavailable(_))
            ));
        }
    }

    #[test]
    fn selection_takes_the_newest_minor_for_the_target_and_honours_a_pin() {
        let bundles = [
            spec("18.6", "aarch64-apple-darwin", SHA_A),
            spec("18.10", "aarch64-apple-darwin", SHA_A),
            spec("18.11", "x86_64-apple-darwin", SHA_A),
            spec("17.11", "aarch64-apple-darwin", SHA_A),
        ];
        let select = |tag: &str| {
            select_newest(
                &bundles,
                "aarch64-apple-darwin",
                &PgVersionRequest::parse(tag).unwrap(),
            )
            .map(|spec| spec.postgres_version.as_str())
        };
        // Numeric, not lexical: 18.10 is newer than 18.6.
        assert_eq!(select("18"), Some("18.10"));
        assert_eq!(select("18.6"), Some("18.6"));
        assert_eq!(select("17"), Some("17.11"));
        assert_eq!(select("18.7"), None);
        assert_eq!(
            select_newest(
                &bundles,
                "aarch64-unknown-linux-musl",
                &PgVersionRequest::major("18")
            ),
            None
        );
    }

    #[test]
    fn cache_key_is_the_bundle_name_and_a_sha256_prefix() {
        let first = spec("18.7", "aarch64-apple-darwin", SHA_A);
        let rebuilt = BundleSpec {
            sha256: SHA_A.replace('a', "b"),
            ..first.clone()
        };
        assert_eq!(
            first.cache_key(),
            "pg-18.7-pgch-0.11.1-aarch64-apple-darwin-aaaaaaaaaaaa"
        );
        assert_ne!(first.cache_key(), rebuilt.cache_key());
    }

    #[test]
    fn manifest_entries_need_a_plain_archive_name_and_a_full_sha256() {
        let valid = spec("18.7", "aarch64-apple-darwin", SHA_A);
        assert_eq!(valid.validate(), Ok(()));
        for file in [
            "../evil.tar.gz",
            "dir/pg.tar.gz",
            ".hidden.tar.gz",
            "pg.zip",
            ".tar.gz",
            "https://example.com/pg.tar.gz",
        ] {
            let spec = BundleSpec {
                file: file.to_string(),
                ..valid.clone()
            };
            assert!(spec.validate().is_err(), "{file}");
        }
        for sha256 in ["", "abc", &SHA_A.replace('a', "g"), &format!("{SHA_A}a")] {
            let spec = BundleSpec {
                sha256: sha256.to_string(),
                ..valid.clone()
            };
            assert!(spec.validate().is_err(), "{sha256}");
        }
    }

    #[test]
    fn checksum_verification_compares_the_file_with_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bundle.tar.gz");
        std::fs::write(&archive, b"abc").unwrap();
        // sha256("abc")
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let matching = spec("18.7", "aarch64-apple-darwin", &digest.to_uppercase());
        verify_sha256(&archive, &matching).unwrap();

        let mismatch = spec("18.7", "aarch64-apple-darwin", SHA_A);
        match verify_sha256(&archive, &mismatch) {
            Err(Error::PostgresBundleChecksum {
                expected, actual, ..
            }) => {
                assert_eq!(expected, SHA_A);
                assert_eq!(actual, digest);
            }
            other => panic!("expected a checksum error, got {other:?}"),
        }
    }

    #[test]
    fn bundle_source_accepts_https_and_file_urls_only() {
        assert_eq!(
            BundleSource::parse(None).unwrap(),
            BundleSource::Https(url::Url::parse(&format!("{DEFAULT_BUNDLES_URL}/")).unwrap())
        );
        let BundleSource::Https(base) =
            BundleSource::parse(Some("https://example.com/bundles/v1")).unwrap()
        else {
            panic!("expected an https source");
        };
        assert_eq!(
            base.join(MANIFEST_FILE).unwrap().as_str(),
            "https://example.com/bundles/v1/manifest.json"
        );
        assert_eq!(
            BundleSource::parse(Some("file:///tmp/bundles")).unwrap(),
            BundleSource::Directory(PathBuf::from("/tmp/bundles/"))
        );
        for url in ["http://example.com", "ftp://example.com", "/tmp/bundles"] {
            assert!(
                matches!(
                    BundleSource::parse(Some(url)),
                    Err(Error::PostgresBundleUnavailable(_))
                ),
                "{url}"
            );
        }
    }

    #[test]
    fn only_complete_cache_entries_count_as_installed() {
        let root = tempfile::tempdir().unwrap();
        let old = spec("18.6", "aarch64-apple-darwin", SHA_A);
        let new = spec("18.7", "aarch64-apple-darwin", SHA_A);
        for spec in [&old, &new] {
            let dir = root.path().join(spec.cache_key());
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(MARKER_FILE), serde_json::to_vec(spec).unwrap()).unwrap();
        }
        // An interrupted extraction and a staging directory have no marker
        // where a bundle keeps it.
        std::fs::create_dir_all(root.path().join("pg-18.9-partial")).unwrap();
        let staging = root.path().join(".staging-1");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(
            staging.join(MARKER_FILE),
            serde_json::to_vec(&spec("18.9", "aarch64-apple-darwin", SHA_A)).unwrap(),
        )
        .unwrap();

        let newest = newest_installed_in(
            root.path(),
            "aarch64-apple-darwin",
            &PgVersionRequest::major("18"),
        )
        .unwrap();
        assert_eq!(newest.spec, new);
        assert_eq!(newest.dir, root.path().join(new.cache_key()));
        assert_eq!(
            newest_installed_in(
                root.path(),
                "aarch64-apple-darwin",
                &PgVersionRequest::major("17")
            ),
            None
        );
    }
}
