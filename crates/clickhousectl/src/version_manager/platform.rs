use crate::error::{Error, Result};
use crate::version_manager::list::Channel;
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

/// Detected platform (OS + architecture)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    /// Detects the current platform
    pub fn detect() -> Result<Self> {
        let os = match std::env::consts::OS {
            "macos" => Os::MacOS,
            "linux" => Os::Linux,
            other => {
                return Err(Error::UnsupportedPlatform {
                    os: other.to_string(),
                    arch: std::env::consts::ARCH.to_string(),
                });
            }
        };

        let arch = match std::env::consts::ARCH {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            other => {
                return Err(Error::UnsupportedPlatform {
                    os: std::env::consts::OS.to_string(),
                    arch: other.to_string(),
                });
            }
        };

        Ok(Platform { os, arch })
    }

    /// builds.clickhouse.com platform path segment (e.g., "amd64", "macos-aarch64")
    pub fn builds_path(&self) -> &'static str {
        match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) => "amd64",
            (Os::Linux, Arch::Aarch64) => "aarch64",
            (Os::MacOS, Arch::X86_64) => "macos",
            (Os::MacOS, Arch::Aarch64) => "macos-aarch64",
        }
    }

    /// packages.clickhouse.com arch string (e.g., "amd64", "arm64")
    /// Only valid for Linux — macOS packages are not available
    pub fn packages_arch(&self) -> Option<&'static str> {
        match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) => Some("amd64"),
            (Os::Linux, Arch::Aarch64) => Some("arm64"),
            (Os::MacOS, _) => None,
        }
    }

    /// GitHub releases arch/suffix for download URLs
    pub fn github_suffix(&self) -> &'static str {
        match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) => "amd64",
            (Os::Linux, Arch::Aarch64) => "arm64",
            (Os::MacOS, Arch::X86_64) => "x86_64",
            (Os::MacOS, Arch::Aarch64) => "aarch64",
        }
    }
}

/// Where to download a ClickHouse binary from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadSource {
    /// builds.clickhouse.com — single binary, no extraction needed on any platform
    Builds {
        /// Version path segment: "master", "25.12", etc.
        version_path: String,
    },
    /// packages.clickhouse.com — tgz tarball (Linux only)
    Packages { channel: Channel, version: String },
    /// GitHub releases — tgz (Linux) or bare binary (macOS)
    GitHub { version: String, channel: Channel },
}

impl DownloadSource {
    /// Construct the full download URL for the given platform
    pub fn url(&self, platform: &Platform) -> String {
        match self {
            DownloadSource::Builds { version_path } => {
                format!(
                    "https://builds.clickhouse.com/{}/{}/clickhouse",
                    version_path,
                    platform.builds_path()
                )
            }
            DownloadSource::Packages { channel, version } => {
                let arch = platform
                    .packages_arch()
                    .expect("Packages source should only be used for Linux");
                format!(
                    "https://packages.clickhouse.com/tgz/{}/clickhouse-common-static-{}-{}.tgz",
                    channel, version, arch
                )
            }
            DownloadSource::GitHub { version, channel } => {
                let base = format!(
                    "https://github.com/ClickHouse/ClickHouse/releases/download/v{}-{}",
                    version, channel
                );
                match platform.os {
                    Os::Linux => {
                        format!(
                            "{}/clickhouse-common-static-{}-{}.tgz",
                            base,
                            version,
                            platform.github_suffix()
                        )
                    }
                    Os::MacOS => {
                        format!("{}/clickhouse-macos-{}", base, platform.github_suffix())
                    }
                }
            }
        }
    }

    /// Whether this source downloads a tarball that needs extraction
    pub fn is_tarball(&self, platform: &Platform) -> bool {
        match self {
            DownloadSource::Builds { .. } => false, // Always a single binary
            DownloadSource::Packages { .. } => true, // Always a tgz
            DownloadSource::GitHub { .. } => platform.os == Os::Linux, // tgz on Linux, binary on macOS
        }
    }
}

/// Construct the HEAD-check URL for probing builds.clickhouse.com availability
pub fn builds_probe_url(version_path: &str, platform: &Platform) -> String {
    format!(
        "https://builds.clickhouse.com/{}/{}/clickhouse",
        version_path,
        platform.builds_path()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Platform detection --

    #[test]
    fn test_detect_platform() {
        let p = Platform::detect().unwrap();
        assert!(p.os == Os::MacOS || p.os == Os::Linux);
        assert!(p.arch == Arch::X86_64 || p.arch == Arch::Aarch64);
    }

    const LINUX_AMD64: Platform = Platform {
        os: Os::Linux,
        arch: Arch::X86_64,
    };
    const LINUX_ARM64: Platform = Platform {
        os: Os::Linux,
        arch: Arch::Aarch64,
    };
    const MACOS_X86: Platform = Platform {
        os: Os::MacOS,
        arch: Arch::X86_64,
    };
    const MACOS_ARM64: Platform = Platform {
        os: Os::MacOS,
        arch: Arch::Aarch64,
    };

    #[test]
    fn every_platform_maps_to_its_builds_packages_and_github_segments() {
        for (platform, builds, packages, github) in [
            (LINUX_AMD64, "amd64", Some("amd64"), "amd64"),
            (LINUX_ARM64, "aarch64", Some("arm64"), "arm64"),
            // macOS packages are not published.
            (MACOS_X86, "macos", None, "x86_64"),
            (MACOS_ARM64, "macos-aarch64", None, "aarch64"),
        ] {
            assert_eq!(platform.builds_path(), builds, "{platform:?}");
            assert_eq!(platform.packages_arch(), packages, "{platform:?}");
            assert_eq!(platform.github_suffix(), github, "{platform:?}");
        }
    }

    #[test]
    fn download_urls_and_tarball_shape_follow_the_source_and_platform() {
        let builds = |version_path: &str| DownloadSource::Builds {
            version_path: version_path.to_string(),
        };
        let packages = |channel, version: &str| DownloadSource::Packages {
            channel,
            version: version.to_string(),
        };
        let github = |version: &str, channel| DownloadSource::GitHub {
            version: version.to_string(),
            channel,
        };
        for (source, platform, url, tarball) in [
            (
                builds("master"),
                LINUX_AMD64,
                "https://builds.clickhouse.com/master/amd64/clickhouse",
                false,
            ),
            (
                builds("master"),
                MACOS_ARM64,
                "https://builds.clickhouse.com/master/macos-aarch64/clickhouse",
                false,
            ),
            (
                builds("25.12"),
                MACOS_ARM64,
                "https://builds.clickhouse.com/25.12/macos-aarch64/clickhouse",
                false,
            ),
            (
                builds("25.8"),
                LINUX_ARM64,
                "https://builds.clickhouse.com/25.8/aarch64/clickhouse",
                false,
            ),
            (
                packages(Channel::Stable, "25.12.9.61"),
                LINUX_AMD64,
                "https://packages.clickhouse.com/tgz/stable/clickhouse-common-static-25.12.9.61-amd64.tgz",
                true,
            ),
            (
                packages(Channel::Lts, "24.8.6.70"),
                LINUX_ARM64,
                "https://packages.clickhouse.com/tgz/lts/clickhouse-common-static-24.8.6.70-arm64.tgz",
                true,
            ),
            // GitHub ships a tarball on Linux and a bare binary on macOS.
            (
                github("25.12.5.44", Channel::Stable),
                LINUX_AMD64,
                "https://github.com/ClickHouse/ClickHouse/releases/download/v25.12.5.44-stable/clickhouse-common-static-25.12.5.44-amd64.tgz",
                true,
            ),
            (
                github("25.12.5.44", Channel::Stable),
                MACOS_ARM64,
                "https://github.com/ClickHouse/ClickHouse/releases/download/v25.12.5.44-stable/clickhouse-macos-aarch64",
                false,
            ),
            (
                github("24.8.6.70", Channel::Lts),
                MACOS_X86,
                "https://github.com/ClickHouse/ClickHouse/releases/download/v24.8.6.70-lts/clickhouse-macos-x86_64",
                false,
            ),
        ] {
            assert_eq!(source.url(&platform), url, "{source:?} {platform:?}");
            assert_eq!(
                source.is_tarball(&platform),
                tarball,
                "{source:?} {platform:?}"
            );
            // The availability probe checks exactly the URL a builds download fetches.
            if let DownloadSource::Builds { version_path } = &source {
                assert_eq!(builds_probe_url(version_path, &platform), url);
            }
        }
    }
}
