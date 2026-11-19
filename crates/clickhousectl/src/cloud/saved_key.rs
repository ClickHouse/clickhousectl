//! The lines that explain a saved project API key (`.clickhouse/credentials.json`)
//! which cannot work, and how to replace or clear it: the saved key outranks
//! the OAuth login, so commands keep sending it until it is replaced.

/// Whether the locally saved OAuth login is usable, by the test `auth status`
/// uses (`load_tokens` + `is_token_valid`).
pub(crate) fn oauth_login_valid() -> bool {
    matches!(
        crate::cloud::auth::load_tokens(),
        Ok(Some(tokens)) if crate::cloud::auth::is_token_valid(&tokens)
    )
}

/// The saved credentials file path, as shown on the Key line.
pub(crate) fn credentials_path_display() -> String {
    crate::cloud::credentials::credentials_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| ".clickhouse/credentials.json".to_string())
}

/// The Key, OAuth login and recovery lines under the first line of the
/// message. `granted` is the `role X` / `roles X, Y` phrase of the key's
/// `cloud access request` grant, when it has one.
pub(crate) fn recovery_lines(path: &str, granted: Option<&str>, oauth_valid: bool) -> Vec<String> {
    let mut key = format!("  Key: {path}");
    if let Some(roles) = granted {
        key.push_str(&format!(" (from `cloud access request`, {roles})"));
    }
    let oauth = if oauth_valid {
        "  OAuth login: valid (read-only)."
    } else {
        "  OAuth login: none valid."
    };
    let mut lines = vec![key, oauth.to_string()];
    if oauth_valid {
        lines.push(format!(
            "  New key: `clickhousectl cloud access roles`, then `{}`",
            crate::cloud::grant::ACCESS_REQUEST
        ));
        lines.push(
            "  Or clear it to use the OAuth login: `clickhousectl cloud auth logout --api-keys`"
                .to_string(),
        );
    } else {
        lines.push(format!(
            "  First log in (needs the user): `clickhousectl cloud auth login`. Then: `{}`",
            crate::cloud::grant::ACCESS_REQUEST
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/home/dev/work/.clickhouse/credentials.json";

    #[test]
    fn recovery_lines_with_a_valid_oauth_login_offer_a_new_key_or_clearing_it() {
        let lines = recovery_lines(PATH, Some("role Service API Reader"), true);
        assert_eq!(
            lines,
            [
                "  Key: /home/dev/work/.clickhouse/credentials.json (from `cloud access request`, role Service API Reader)",
                "  OAuth login: valid (read-only).",
                "  New key: `clickhousectl cloud access roles`, then `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`",
                "  Or clear it to use the OAuth login: `clickhousectl cloud auth logout --api-keys`",
            ]
        );
    }

    #[test]
    fn recovery_lines_without_a_valid_oauth_login_ask_for_a_login_first() {
        let lines = recovery_lines(PATH, None, false);
        assert_eq!(
            lines,
            [
                "  Key: /home/dev/work/.clickhouse/credentials.json",
                "  OAuth login: none valid.",
                "  First log in (needs the user): `clickhousectl cloud auth login`. Then: `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`",
            ]
        );
    }
}
