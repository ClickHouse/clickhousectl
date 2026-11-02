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
        return Err(Error::InvalidVersion("empty version".to_string()));
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
            return Err(Error::InvalidVersion(format!(
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
        3 => Err(Error::InvalidVersion(format!(
            "3-part version '{}' is not supported. Use a full 4-part version (e.g., {}.1)",
            input, input
        ))),
        4 => Ok(VersionSpec::Exact(input.to_string())),
        _ => Err(Error::InvalidVersion(format!(
            "invalid version '{}': expected 1-2 or 4 parts",
            input
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_spec_reads_keywords_and_numeric_shapes_and_displays_them_back() {
        for (input, expected) in [
            ("latest", VersionSpec::Latest),
            ("stable", VersionSpec::Channel(Channel::Stable)),
            ("lts", VersionSpec::Channel(Channel::Lts)),
            ("25", VersionSpec::Major(25)),
            ("25.12", VersionSpec::Minor(25, 12)),
            ("25.12.9.61", VersionSpec::Exact("25.12.9.61".to_string())),
            ("  stable  ", VersionSpec::Channel(Channel::Stable)),
            (" 25.12 ", VersionSpec::Minor(25, 12)),
        ] {
            assert_eq!(parse_version_spec(input).unwrap(), expected, "{input:?}");
            assert_eq!(expected.to_string(), input.trim(), "{input:?}");
        }
    }

    #[test]
    fn parse_version_spec_rejects_other_shapes_naming_the_input() {
        // Each message quotes the input; a 3-part one also suggests the
        // 4-part form to use instead.
        for (input, identifiers) in [
            ("25.12.9", &["'25.12.9'", "25.12.9.1"][..]),
            ("25.12.9.61.2", &["'25.12.9.61.2'"][..]),
            ("", &[][..]),
            ("foo", &["'foo'"][..]),
            ("25.foo", &["'25.foo'"][..]),
            ("abc.12.9.61", &["'abc.12.9.61'"][..]),
        ] {
            let err = parse_version_spec(input).unwrap_err();
            assert!(matches!(&err, Error::InvalidVersion(_)), "{input:?}");
            let message = err.to_string();
            for identifier in identifiers {
                assert!(message.contains(identifier), "{input:?}: {message}");
            }
        }
    }
}
