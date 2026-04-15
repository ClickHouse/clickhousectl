use crate::error::{Error, Result};
use crate::version_manager::list::Channel;
use std::fmt;

/// Parsed version specification from user input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSpec {
    /// `latest` — bleeding-edge master build
    Latest,
    /// `stable` or `lts` — resolve via GitHub API to a minor version
    Channel(Channel),
    /// `25` — resolve to highest available minor in that major
    Major(u32),
    /// `25.12` — download directly from builds
    Minor(u32, u32),
    /// `25.12.9.61` — exact 4-part version
    Exact(String),
}

impl fmt::Display for VersionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionSpec::Latest => write!(f, "latest"),
            VersionSpec::Channel(ch) => write!(f, "{}", ch),
            VersionSpec::Major(major) => write!(f, "{}", major),
            VersionSpec::Minor(major, minor) => write!(f, "{}.{}", major, minor),
            VersionSpec::Exact(v) => write!(f, "{}", v),
        }
    }
}

/// Parse a user-provided version string into a VersionSpec
pub fn parse_version_spec(input: &str) -> Result<VersionSpec> {
    let input = input.trim();

    if input.is_empty() {
        return Err(Error::NoMatchingVersion("empty version".to_string()));
    }

    // Keywords
    match input {
        "latest" => return Ok(VersionSpec::Latest),
        "stable" => return Ok(VersionSpec::Channel(Channel::Stable)),
        "lts" => return Ok(VersionSpec::Channel(Channel::Lts)),
        _ => {}
    }

    // Numeric version parts
    let parts: Vec<&str> = input.split('.').collect();

    // Validate all parts are numeric
    for part in &parts {
        if part.parse::<u32>().is_err() {
            return Err(Error::NoMatchingVersion(format!(
                "invalid version '{}': all parts must be numeric",
                input
            )));
        }
    }

    match parts.len() {
        1 => {
            let major: u32 = parts[0].parse().unwrap();
            Ok(VersionSpec::Major(major))
        }
        2 => {
            let major: u32 = parts[0].parse().unwrap();
            let minor: u32 = parts[1].parse().unwrap();
            Ok(VersionSpec::Minor(major, minor))
        }
        3 => Err(Error::NoMatchingVersion(format!(
            "3-part version '{}' is not supported. Use a full 4-part version (e.g., {}.1)",
            input, input
        ))),
        4 => Ok(VersionSpec::Exact(input.to_string())),
        _ => Err(Error::NoMatchingVersion(format!(
            "invalid version '{}': expected 1-2 or 4 parts",
            input
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_latest() {
        assert_eq!(parse_version_spec("latest").unwrap(), VersionSpec::Latest);
    }

    #[test]
    fn test_parse_stable() {
        assert_eq!(
            parse_version_spec("stable").unwrap(),
            VersionSpec::Channel(Channel::Stable)
        );
    }

    #[test]
    fn test_parse_lts() {
        assert_eq!(
            parse_version_spec("lts").unwrap(),
            VersionSpec::Channel(Channel::Lts)
        );
    }

    #[test]
    fn test_parse_major() {
        assert_eq!(parse_version_spec("25").unwrap(), VersionSpec::Major(25));
    }

    #[test]
    fn test_parse_minor() {
        assert_eq!(
            parse_version_spec("25.12").unwrap(),
            VersionSpec::Minor(25, 12)
        );
    }

    #[test]
    fn test_parse_exact() {
        assert_eq!(
            parse_version_spec("25.12.9.61").unwrap(),
            VersionSpec::Exact("25.12.9.61".to_string())
        );
    }

    #[test]
    fn test_parse_3_part_rejected() {
        let err = parse_version_spec("25.12.9").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("3-part version"), "got: {}", msg);
        assert!(msg.contains("25.12.9.1"), "should hint at 4-part: {}", msg);
    }

    #[test]
    fn test_parse_5_part_rejected() {
        assert!(parse_version_spec("25.12.9.61.2").is_err());
    }

    #[test]
    fn test_parse_empty_rejected() {
        assert!(parse_version_spec("").is_err());
    }

    #[test]
    fn test_parse_non_numeric_rejected() {
        assert!(parse_version_spec("foo").is_err());
        assert!(parse_version_spec("25.foo").is_err());
        assert!(parse_version_spec("abc.12.9.61").is_err());
    }

    #[test]
    fn test_parse_whitespace_trimmed() {
        assert_eq!(
            parse_version_spec("  stable  ").unwrap(),
            VersionSpec::Channel(Channel::Stable)
        );
        assert_eq!(
            parse_version_spec(" 25.12 ").unwrap(),
            VersionSpec::Minor(25, 12)
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(VersionSpec::Latest.to_string(), "latest");
        assert_eq!(VersionSpec::Channel(Channel::Stable).to_string(), "stable");
        assert_eq!(VersionSpec::Channel(Channel::Lts).to_string(), "lts");
        assert_eq!(VersionSpec::Major(25).to_string(), "25");
        assert_eq!(VersionSpec::Minor(25, 12).to_string(), "25.12");
        assert_eq!(
            VersionSpec::Exact("25.12.9.61".to_string()).to_string(),
            "25.12.9.61"
        );
    }
}
