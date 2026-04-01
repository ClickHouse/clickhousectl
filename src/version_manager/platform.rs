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
    Packages {
        channel: Channel,
        version: String,
    },
    /// GitHub releases — tgz (Linux) or bare binary (macOS)
    GitHub {
        version: String,
        channel: Channel,
    },
    /// packages.clickhouse.com client-only tarball (Linux only, ~36KB)
    PackagesClient {
        channel: Channel,
        version: String,
    },
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
                        format!(
                            "{}/clickhouse-macos-{}",
                            base,
                            platform.github_suffix()
                        )
                    }
                }
            }
            DownloadSource::PackagesClient { channel, version } => {
                let arch = platform
                    .packages_arch()
                    .expect("PackagesClient source should only be used for Linux");
                format!(
                    "https://packages.clickhouse.com/tgz/{}/clickhouse-client-{}-{}.tgz",
                    channel, version, arch
                )
            }
        }
    }

    /// Whether this source downloads a tarball that needs extraction
    pub fn is_tarball(&self, platform: &Platform) -> bool {
        match self {
            DownloadSource::Builds { .. } => false,
            DownloadSource::Packages { .. } => true,
            DownloadSource::GitHub { .. } => platform.os == Os::Linux,
            DownloadSource::PackagesClient { .. } => true,
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

    // -- builds.clickhouse.com paths --

    #[test]
    fn test_builds_path_linux_amd64() {
        let p = Platform { os: Os::Linux, arch: Arch::X86_64 };
        assert_eq!(p.builds_path(), "amd64");
    }

    #[test]
    fn test_builds_path_linux_aarch64() {
        let p = Platform { os: Os::Linux, arch: Arch::Aarch64 };
        assert_eq!(p.builds_path(), "aarch64");
    }

    #[test]
    fn test_builds_path_macos_x86() {
        let p = Platform { os: Os::MacOS, arch: Arch::X86_64 };
        assert_eq!(p.builds_path(), "macos");
    }

    #[test]
    fn test_builds_path_macos_aarch64() {
        let p = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        assert_eq!(p.builds_path(), "macos-aarch64");
    }

    // -- packages.clickhouse.com arch --

    #[test]
    fn test_packages_arch_linux() {
        let p = Platform { os: Os::Linux, arch: Arch::X86_64 };
        assert_eq!(p.packages_arch(), Some("amd64"));
        let p = Platform { os: Os::Linux, arch: Arch::Aarch64 };
        assert_eq!(p.packages_arch(), Some("arm64"));
    }

    #[test]
    fn test_packages_arch_macos_none() {
        let p = Platform { os: Os::MacOS, arch: Arch::X86_64 };
        assert_eq!(p.packages_arch(), None);
        let p = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        assert_eq!(p.packages_arch(), None);
    }

    // -- GitHub suffix --

    #[test]
    fn test_github_suffix() {
        assert_eq!(Platform { os: Os::Linux, arch: Arch::X86_64 }.github_suffix(), "amd64");
        assert_eq!(Platform { os: Os::Linux, arch: Arch::Aarch64 }.github_suffix(), "arm64");
        assert_eq!(Platform { os: Os::MacOS, arch: Arch::X86_64 }.github_suffix(), "x86_64");
        assert_eq!(Platform { os: Os::MacOS, arch: Arch::Aarch64 }.github_suffix(), "aarch64");
    }

    // -- URL construction: builds.clickhouse.com --

    #[test]
    fn test_builds_url_master_linux_amd64() {
        let p = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let src = DownloadSource::Builds { version_path: "master".to_string() };
        assert_eq!(src.url(&p), "https://builds.clickhouse.com/master/amd64/clickhouse");
    }

    #[test]
    fn test_builds_url_minor_macos_aarch64() {
        let p = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        let src = DownloadSource::Builds { version_path: "25.12".to_string() };
        assert_eq!(
            src.url(&p),
            "https://builds.clickhouse.com/25.12/macos-aarch64/clickhouse"
        );
    }

    #[test]
    fn test_builds_url_minor_linux_aarch64() {
        let p = Platform { os: Os::Linux, arch: Arch::Aarch64 };
        let src = DownloadSource::Builds { version_path: "25.8".to_string() };
        assert_eq!(
            src.url(&p),
            "https://builds.clickhouse.com/25.8/aarch64/clickhouse"
        );
    }

    // -- URL construction: packages.clickhouse.com --

    #[test]
    fn test_packages_url_stable_linux_amd64() {
        let p = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let src = DownloadSource::Packages {
            channel: Channel::Stable,
            version: "25.12.9.61".to_string(),
        };
        assert_eq!(
            src.url(&p),
            "https://packages.clickhouse.com/tgz/stable/clickhouse-common-static-25.12.9.61-amd64.tgz"
        );
    }

    #[test]
    fn test_packages_url_lts_linux_arm64() {
        let p = Platform { os: Os::Linux, arch: Arch::Aarch64 };
        let src = DownloadSource::Packages {
            channel: Channel::Lts,
            version: "24.8.6.70".to_string(),
        };
        assert_eq!(
            src.url(&p),
            "https://packages.clickhouse.com/tgz/lts/clickhouse-common-static-24.8.6.70-arm64.tgz"
        );
    }

    // -- URL construction: GitHub releases --

    #[test]
    fn test_github_url_linux_amd64() {
        let p = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let src = DownloadSource::GitHub {
            version: "25.12.5.44".to_string(),
            channel: Channel::Stable,
        };
        assert_eq!(
            src.url(&p),
            "https://github.com/ClickHouse/ClickHouse/releases/download/v25.12.5.44-stable/clickhouse-common-static-25.12.5.44-amd64.tgz"
        );
    }

    #[test]
    fn test_github_url_macos_aarch64() {
        let p = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        let src = DownloadSource::GitHub {
            version: "25.12.5.44".to_string(),
            channel: Channel::Stable,
        };
        assert_eq!(
            src.url(&p),
            "https://github.com/ClickHouse/ClickHouse/releases/download/v25.12.5.44-stable/clickhouse-macos-aarch64"
        );
    }

    #[test]
    fn test_github_url_macos_x86() {
        let p = Platform { os: Os::MacOS, arch: Arch::X86_64 };
        let src = DownloadSource::GitHub {
            version: "24.8.6.70".to_string(),
            channel: Channel::Lts,
        };
        assert_eq!(
            src.url(&p),
            "https://github.com/ClickHouse/ClickHouse/releases/download/v24.8.6.70-lts/clickhouse-macos-x86_64"
        );
    }

    // -- Tarball detection --

    #[test]
    fn test_builds_never_tarball() {
        let linux = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let macos = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        let src = DownloadSource::Builds { version_path: "master".to_string() };
        assert!(!src.is_tarball(&linux));
        assert!(!src.is_tarball(&macos));
    }

    #[test]
    fn test_packages_always_tarball() {
        let linux = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let src = DownloadSource::Packages {
            channel: Channel::Stable,
            version: "25.12.9.61".to_string(),
        };
        assert!(src.is_tarball(&linux));
    }

    #[test]
    fn test_github_tarball_linux_only() {
        let linux = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let macos = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        let src = DownloadSource::GitHub {
            version: "25.12.5.44".to_string(),
            channel: Channel::Stable,
        };
        assert!(src.is_tarball(&linux));
        assert!(!src.is_tarball(&macos));
    }

    // -- URL construction: packages client-only --

    #[test]
    fn test_packages_client_url_stable_linux_amd64() {
        let p = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let src = DownloadSource::PackagesClient {
            channel: Channel::Stable,
            version: "25.3.2.39".to_string(),
        };
        assert_eq!(
            src.url(&p),
            "https://packages.clickhouse.com/tgz/stable/clickhouse-client-25.3.2.39-amd64.tgz"
        );
    }

    #[test]
    fn test_packages_client_url_lts_linux_arm64() {
        let p = Platform { os: Os::Linux, arch: Arch::Aarch64 };
        let src = DownloadSource::PackagesClient {
            channel: Channel::Lts,
            version: "24.8.6.70".to_string(),
        };
        assert_eq!(
            src.url(&p),
            "https://packages.clickhouse.com/tgz/lts/clickhouse-client-24.8.6.70-arm64.tgz"
        );
    }

    #[test]
    fn test_packages_client_always_tarball() {
        let linux = Platform { os: Os::Linux, arch: Arch::X86_64 };
        let src = DownloadSource::PackagesClient {
            channel: Channel::Stable,
            version: "25.3.2.39".to_string(),
        };
        assert!(src.is_tarball(&linux));
    }

    // -- Probe URL --

    #[test]
    fn test_builds_probe_url() {
        let p = Platform { os: Os::MacOS, arch: Arch::Aarch64 };
        assert_eq!(
            builds_probe_url("25.12", &p),
            "https://builds.clickhouse.com/25.12/macos-aarch64/clickhouse"
        );
    }
}
