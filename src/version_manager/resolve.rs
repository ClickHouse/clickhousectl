use crate::error::{Error, Result};
use crate::version_manager::list::{list_available_versions, VersionEntry};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOS,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Os::MacOS => write!(f, "macos"),
            Os::Linux => write!(f, "linux"),
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::Aarch64 => write!(f, "aarch64"),
        }
    }
}

/// Detects the current platform
pub fn detect_platform() -> Result<(Os, Arch)> {
    let os = match std::env::consts::OS {
        "macos" => Os::MacOS,
        "linux" => Os::Linux,
        other => {
            return Err(Error::UnsupportedPlatform {
                os: other.to_string(),
                arch: std::env::consts::ARCH.to_string(),
            })
        }
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        other => {
            return Err(Error::UnsupportedPlatform {
                os: std::env::consts::OS.to_string(),
                arch: other.to_string(),
            })
        }
    };

    Ok((os, arch))
}

/// Resolves a version specifier to an exact version and its channel
/// Supports:
/// - Exact: "25.1.2.3" -> ("25.1.2.3", "stable") (assumes stable for exact versions)
/// - Partial: "25.1" -> latest matching "25.1.x.x" with its actual channel
/// - Channel: "stable" -> latest stable, "lts" -> latest lts
pub async fn resolve_version(version_spec: &str) -> Result<VersionEntry> {
    // For all specifiers, fetch available versions to get accurate channel info
    let available = list_available_versions().await?;

    // If it looks like an exact version (4 parts), find its channel from the list
    if version_spec.split('.').count() == 4 {
        let channel = available
            .iter()
            .find(|e| e.version == version_spec)
            .map(|e| e.channel.clone())
            .unwrap_or_else(|| "stable".to_string());
        return Ok(VersionEntry {
            version: version_spec.to_string(),
            channel,
        });
    }

    match version_spec {
        "stable" => {
            available
                .iter()
                .find(|e| e.channel == "stable")
                .cloned()
                .ok_or_else(|| Error::NoMatchingVersion(version_spec.to_string()))
        }
        "lts" => {
            available
                .iter()
                .find(|e| e.channel == "lts")
                .cloned()
                .ok_or_else(|| Error::NoMatchingVersion(version_spec.to_string()))
        }
        partial => {
            // Find the latest version matching the partial spec
            let prefix = format!("{}.", partial);
            available
                .iter()
                .find(|e| e.version.starts_with(&prefix) || e.version == partial)
                .cloned()
                .ok_or_else(|| Error::NoMatchingVersion(partial.to_string()))
        }
    }
}

/// Returns whether the current platform downloads a tarball (Linux) vs a bare binary (macOS)
pub fn is_tarball_download() -> Result<bool> {
    let (os, _) = detect_platform()?;
    Ok(os == Os::Linux)
}

/// Builds the download URL for a specific version from GitHub releases
/// macOS: .../clickhouse-macos-{arch}
/// Linux: .../clickhouse-common-static-{version}-{arch}.tgz
pub fn build_download_url(version: &str, channel: &str) -> Result<String> {
    let (os, arch) = detect_platform()?;
    let base = format!(
        "https://github.com/ClickHouse/ClickHouse/releases/download/v{}-{}",
        version, channel
    );
    match os {
        Os::Linux => {
            let linux_arch = match arch {
                Arch::X86_64 => "amd64",
                Arch::Aarch64 => "arm64",
            };
            Ok(format!(
                "{}/clickhouse-common-static-{}-{}.tgz",
                base, version, linux_arch
            ))
        }
        Os::MacOS => Ok(format!("{}/clickhouse-{}-{}", base, os, arch)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        let result = detect_platform();
        assert!(result.is_ok());
        let (os, arch) = result.unwrap();
        assert!(os == Os::MacOS || os == Os::Linux);
        assert!(arch == Arch::X86_64 || arch == Arch::Aarch64);
    }

    #[test]
    fn test_build_download_url_stable() {
        let url = build_download_url("25.12.5.44", "stable").unwrap();
        assert!(url.starts_with("https://github.com/ClickHouse/ClickHouse/releases/download/"));
        assert!(url.contains("v25.12.5.44-stable"));
    }

    #[test]
    fn test_build_download_url_lts() {
        let url = build_download_url("25.8.16.34", "lts").unwrap();
        assert!(url.starts_with("https://github.com/ClickHouse/ClickHouse/releases/download/"));
        assert!(url.contains("v25.8.16.34-lts"));
    }

    #[test]
    fn test_build_download_url_platform_specific() {
        let url = build_download_url("25.12.5.44", "stable").unwrap();
        let (os, arch) = detect_platform().unwrap();
        match os {
            Os::MacOS => {
                assert!(url.contains(&format!("clickhouse-macos-{}", arch)));
                assert!(!url.ends_with(".tgz"));
            }
            Os::Linux => {
                let expected_arch = match arch {
                    Arch::X86_64 => "amd64",
                    Arch::Aarch64 => "arm64",
                };
                assert!(url.contains(&format!(
                    "clickhouse-common-static-25.12.5.44-{}.tgz",
                    expected_arch
                )));
            }
        }
    }

    #[test]
    fn test_is_tarball_download() {
        let is_tarball = is_tarball_download().unwrap();
        let (os, _) = detect_platform().unwrap();
        assert_eq!(is_tarball, os == Os::Linux);
    }
}
