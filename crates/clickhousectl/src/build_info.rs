//! Compile-time build identity: version string and experimental build marker.

/// Marker embedded only in `experimental` builds so CI can verify that
/// standard release binaries never include experimental features and
/// experimental binaries always do.
#[cfg(feature = "experimental")]
#[used]
static EXPERIMENTAL_BUILD_MARKER: [u8; 32] = *b"CLICKHOUSECTL_EXPERIMENTAL_BUILD";

/// Returns `true` when `haystack` contains the ASCII substring `needle`.
#[cfg(any(feature = "experimental", test))]
const fn contains_ascii(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    if h.len() < n.len() {
        return false;
    }
    let mut i = 0;
    while i <= h.len() - n.len() {
        let mut j = 0;
        while j < n.len() && h[i + j] == n[j] {
            j += 1;
        }
        if j == n.len() {
            return true;
        }
        i += 1;
    }
    false
}

/// Returns `true` when `value` starts with `prefix`.
#[cfg(any(feature = "experimental", test))]
const fn starts_with_ascii(value: &str, prefix: &str) -> bool {
    let v = value.as_bytes();
    let p = prefix.as_bytes();
    if v.len() < p.len() {
        return false;
    }
    let mut i = 0;
    while i < p.len() {
        if v[i] != p[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns `true` when `value` contains only printable non-whitespace ASCII.
#[cfg(any(feature = "experimental", test))]
const fn is_printable_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b <= b' ' || b >= 0x7f {
            return false;
        }
        i += 1;
    }
    true
}

/// Validates an experimental build version string: must start with the crate
/// version, contain `experimental`, and be a single printable ASCII token.
#[cfg(any(feature = "experimental", test))]
const fn is_valid_experimental_version(value: &str, pkg_version: &str) -> bool {
    is_printable_token(value)
        && starts_with_ascii(value, pkg_version)
        && contains_ascii(value, "experimental")
}

/// Version reported by `--version`, `User-Agent`, telemetry, feedback, debug
/// logs, and Docker container labels.
///
/// Normal builds always report `CARGO_PKG_VERSION` (`X.Y.Z`), ignoring any
/// compile-time environment overrides.
///
/// Experimental builds (`--features experimental`) report
/// `CLICKHOUSECTL_EXPERIMENTAL_VERSION` when set at compile time (used by
/// `experimental-release.yml` to embed the build date and commit SHA, e.g.
/// `0.6.1-experimental.20261128.a1b2c3d`), or `<CARGO_PKG_VERSION>-experimental`
/// for local builds. A compile-time assertion ensures every experimental binary
/// identifies itself as experimental.
#[cfg(feature = "experimental")]
pub const VERSION: &str = {
    let version = match option_env!("CLICKHOUSECTL_EXPERIMENTAL_VERSION") {
        Some(v) if !v.is_empty() => v,
        _ => concat!(env!("CARGO_PKG_VERSION"), "-experimental"),
    };
    assert!(
        is_valid_experimental_version(version, env!("CARGO_PKG_VERSION")),
        "CLICKHOUSECTL_EXPERIMENTAL_VERSION must start with CARGO_PKG_VERSION, contain \"experimental\", and have no whitespace"
    );
    version
};

#[cfg(not(feature = "experimental"))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_build_channel() {
        let pkg = env!("CARGO_PKG_VERSION");
        assert!(VERSION.starts_with(pkg));
        if cfg!(feature = "experimental") {
            assert!(VERSION.contains("experimental"), "got: {VERSION}");
        } else {
            assert_eq!(VERSION, pkg);
        }
    }

    #[test]
    fn experimental_version_validator_accepts_well_formed_values() {
        assert!(is_valid_experimental_version("0.6.1-experimental", "0.6.1"));
        assert!(is_valid_experimental_version(
            "0.6.1-experimental.20261128.a1b2c3d",
            "0.6.1"
        ));
    }

    #[test]
    fn experimental_version_validator_rejects_malformed_values() {
        for bad in [
            "",
            "0.6.1",
            "0.6.1-rc1",
            "0.6.0-experimental",
            "experimental",
            "0.6.1-experimental extra",
            "0.6.1-experimental\n",
        ] {
            assert!(
                !is_valid_experimental_version(bad, "0.6.1"),
                "should reject {bad:?}"
            );
        }
    }
}
