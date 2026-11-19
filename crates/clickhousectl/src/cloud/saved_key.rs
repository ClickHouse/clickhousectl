//! The lines that explain a saved project API key (`.clickhouse/credentials.json`)
//! which cannot work, and how to replace or clear it: the saved key outranks
//! the OAuth login, so commands keep sending it until it is replaced. Used for
//! a key whose grant is known to have expired and for a 401 under the key.

use crate::cloud::client::{AuthSource, CloudClient, CloudError, CloudErrorKind};

/// The API's plain refusal of the credentials themselves (HTTP 401).
const API_REFUSAL: &str = "Invalid credentials";

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
    // Without `cloud access` the key can only be cleared, or the user log in.
    #[cfg(not(feature = "experimental-access-negotiation"))]
    lines.push(if oauth_valid {
        "  Clear it to use the OAuth login: `clickhousectl cloud auth logout --api-keys`"
            .to_string()
    } else {
        "  Log in (needs the user): `clickhousectl cloud auth login`.".to_string()
    });
    #[cfg(feature = "experimental-access-negotiation")]
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

/// The message for the API's 401 to a request sent with the saved key.
pub(crate) fn unauthorized_message(path: &str, granted: Option<&str>, oauth_valid: bool) -> String {
    let mut lines = vec![format!(
        "{API_REFUSAL}: the API refused the saved API key for this project (expired, disabled or \
         deleted). It takes priority over the OAuth login here."
    )];
    lines.extend(recovery_lines(path, granted, oauth_valid));
    lines.join("\n")
}

/// Rewrite the API's plain credential refusal (HTTP 401 `Invalid
/// credentials`) to a request authenticated with the saved credentials file
/// into [`unauthorized_message`]. Every other error, including a 401 under
/// CLI flags, env vars or OAuth, or one with its own reason, is returned
/// unchanged; the kind, exit code and classification are kept.
pub(crate) fn explain_unauthorized(client: &CloudClient, error: CloudError) -> CloudError {
    if client.auth_source() != AuthSource::CredentialsFile
        || error.kind != CloudErrorKind::Auth
        || error.failure.and_then(|failure| failure.http_status) != Some(401)
        || error.message.trim() != API_REFUSAL
    {
        return error;
    }
    #[cfg(feature = "experimental-access-negotiation")]
    let granted =
        crate::cloud::grant::active_grant(client.auth_source()).map(|grant| grant.roles_phrase());
    #[cfg(not(feature = "experimental-access-negotiation"))]
    let granted: Option<String> = None;
    let message = unauthorized_message(
        &credentials_path_display(),
        granted.as_deref(),
        oauth_login_valid(),
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

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/home/dev/work/.clickhouse/credentials.json";

    #[cfg(feature = "experimental-access-negotiation")]
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

    #[cfg(feature = "experimental-access-negotiation")]
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

    #[cfg(not(feature = "experimental-access-negotiation"))]
    #[test]
    fn recovery_lines_without_cloud_access_only_clear_the_key_or_log_in() {
        assert_eq!(
            recovery_lines(PATH, None, true),
            [
                "  Key: /home/dev/work/.clickhouse/credentials.json",
                "  OAuth login: valid (read-only).",
                "  Clear it to use the OAuth login: `clickhousectl cloud auth logout --api-keys`",
            ]
        );
        assert_eq!(
            recovery_lines(PATH, None, false),
            [
                "  Key: /home/dev/work/.clickhouse/credentials.json",
                "  OAuth login: none valid.",
                "  Log in (needs the user): `clickhousectl cloud auth login`.",
            ]
        );
    }

    #[test]
    fn unauthorized_message_leads_with_the_refusal_and_the_key_priority() {
        let message = unauthorized_message(PATH, None, true);
        let lines: Vec<_> = message.lines().collect();
        assert_eq!(
            lines[0],
            "Invalid credentials: the API refused the saved API key for this project (expired, disabled or deleted). It takes priority over the OAuth login here."
        );
        assert_eq!(lines[1..], recovery_lines(PATH, None, true));
        let granted = unauthorized_message(PATH, Some("roles A, B"), false);
        assert!(
            granted.contains(
                "  Key: /home/dev/work/.clickhouse/credentials.json (from `cloud access request`, roles A, B)\n"
            ),
            "{granted}"
        );
        assert!(
            granted.contains("  OAuth login: none valid.\n"),
            "{granted}"
        );
    }

    #[test]
    fn only_the_plain_401_under_the_saved_key_is_rewritten() {
        use crate::failure::{ApiFailure, FailureKind};
        let refused = |status: u16, message: &str| {
            CloudError::auth(message)
                .with_failure(ApiFailure::with_status(FailureKind::Http4xx, status))
        };
        // A 401 under CLI flags is kept.
        let flags = CloudClient::for_tests("http://localhost:1/v1", None);
        let kept = explain_unauthorized(&flags, refused(401, "Invalid credentials"));
        assert_eq!(kept.message, "Invalid credentials");
        let saved = CloudClient::for_tests_saved_key("http://localhost:1/v1");
        // A 401 with its own reason, and other statuses, are kept.
        let own = explain_unauthorized(&saved, refused(401, "UNAUTHORIZED: invalid code"));
        assert_eq!(own.message, "UNAUTHORIZED: invalid code");
        let other = explain_unauthorized(&saved, refused(403, "Invalid credentials"));
        assert_eq!(other.message, "Invalid credentials");
        // The plain refusal under the saved key is rewritten, keeping the
        // kind and classification.
        let rewritten = explain_unauthorized(&saved, refused(401, " Invalid credentials "));
        assert!(
            rewritten
                .message
                .starts_with("Invalid credentials: the API refused the saved API key"),
            "{}",
            rewritten.message
        );
        assert_eq!(rewritten.kind, CloudErrorKind::Auth);
        assert_eq!(
            rewritten.failure.and_then(|failure| failure.http_status),
            Some(401)
        );
    }
}
