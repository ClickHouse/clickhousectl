//! The grant saved with a key from `cloud access request`, and the messages
//! built from it: `auth status`, a bare `FORBIDDEN` under that key, and the
//! empty `clickhouse list` note. Compiled only with
//! `experimental-access-negotiation`.

use crate::cloud::client::{AuthSource, CloudClient, CloudError};
use crate::cloud::permissions::DeclaredNeeds;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

/// What `cloud access request` was granted, saved beside the key it issued
/// (`access_grant` in `credentials.json`). Anything else that writes the saved
/// key drops it, so it always describes the saved key.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AccessGrant {
    /// The saved `api_key` this grant belongs to (the key's public identifier).
    pub api_key: String,
    /// The key's resource id (`keyId`).
    pub key_id: String,
    pub organization_id: String,
    pub request_id: String,
    /// `keyExpiresAt` as RFC 3339 UTC, when the server reported one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub roles: Vec<GrantedRole>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GrantedRole {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `None` when the role lookup failed or did not list this role, so what
    /// the role allows is unknown (never read as "allows nothing").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policies: Option<Vec<GrantPolicy>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GrantPolicy {
    /// `ALLOW` or `DENY` (or an effect this CLI does not know).
    pub effect: String,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
}

impl GrantedRole {
    /// The role's name, else its id.
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }

    /// Whether an ALLOW policy of this role names `permission` (resource
    /// scoping ignored); `None` when the role's policies are unknown.
    fn allows(&self, permission: &str) -> Option<bool> {
        let policies = self.policies.as_ref()?;
        Some(policies.iter().any(|policy| {
            policy.effect == "ALLOW"
                && policy
                    .permissions
                    .iter()
                    .any(|granted| permission_matches(granted, permission))
        }))
    }
}

/// A granted permission covers `needed` when equal, or when it is a `*`
/// wildcard whose prefix `needed` starts with.
fn permission_matches(granted: &str, needed: &str) -> bool {
    match granted.strip_suffix('*') {
        Some(prefix) => needed.starts_with(prefix),
        None => granted == needed,
    }
}

impl AccessGrant {
    /// Role names (ids when unknown), comma-separated.
    pub fn role_labels(&self) -> String {
        self.roles
            .iter()
            .map(GrantedRole::label)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn expires(&self) -> Option<Timestamp> {
        self.expires_at.as_deref()?.parse().ok()
    }

    /// Whether any granted role ALLOWs `permission`: `Some(true)` as soon as a
    /// known role does, `None` when none does but some role is unknown.
    pub fn allows(&self, permission: &str) -> Option<bool> {
        let mut unknown = false;
        for role in &self.roles {
            match role.allows(permission) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => unknown = true,
            }
        }
        (!unknown).then_some(false)
    }

    /// The `role X` / `roles X, Y` phrase.
    pub fn roles_phrase(&self) -> String {
        let noun = if self.roles.len() == 1 {
            "role"
        } else {
            "roles"
        };
        format!("{noun} {}", self.role_labels())
    }
}

pub use crate::cloud::auth::expiry_phrase;

/// The grant saved for the active credential, when that credential is the
/// saved API key and the grant belongs to it.
pub fn active_grant(source: AuthSource) -> Option<AccessGrant> {
    if source != AuthSource::CredentialsFile {
        return None;
    }
    let credentials = crate::cloud::credentials::try_load_credentials().ok()??;
    grant_of(&credentials).cloned()
}

/// The saved grant, when it belongs to the saved key pair.
pub fn grant_of(credentials: &crate::cloud::credentials::Credentials) -> Option<&AccessGrant> {
    let (key, _) = credentials.api_credentials()?;
    credentials
        .access_grant
        .as_ref()
        .filter(|grant| grant.api_key == key)
}

/// `auth status` Scope cell of the saved key: `<roles>, expires HH:MMZ`, or
/// only the roles once expired (the Status cell then names the expiry).
pub fn status_scope(grant: &AccessGrant, now: Timestamp) -> String {
    match grant.expires() {
        Some(expires) if expires > now => {
            format!("{}, {}", grant.role_labels(), expiry_phrase(expires, now))
        }
        _ => grant.role_labels(),
    }
}

/// When the saved key's grant expired (`%Y-%m-%dT%H:%MZ`), once it is past.
pub fn status_expired(grant: &AccessGrant, now: Timestamp) -> Option<String> {
    grant
        .expires()
        .filter(|expires| *expires <= now)
        .map(|expires| {
            expires
                .to_zoned_datetime(TimeZone::UTC)
                .strftime("%Y-%m-%dT%H:%MZ")
                .to_string()
        })
}

/// `auth status` JSON `write_access` of the saved key.
pub fn status_write_access(grant: &AccessGrant, now: Timestamp) -> String {
    let mut text = format!("API key from cloud access request: {}", grant.role_labels());
    if let Some(expires) = grant.expires() {
        let verb = if expires <= now { "expired" } else { "expires" };
        text.push_str(&format!("; {verb} {expires:.0}"));
    }
    text
}

/// The `cloud access request` command line every recovery hint prints.
macro_rules! access_request {
    () => {
        "clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\""
    };
}

/// [`access_request!`] as a constant, for messages built at run time.
pub(crate) const ACCESS_REQUEST: &str = access_request!();

const FIX_LINE: &str = concat!(
    "  Fix: `clickhousectl cloud access roles`, then `",
    access_request!(),
    "`."
);

/// The message for a command that would send the saved key after its grant
/// expired (`expired`, `%Y-%m-%dT%H:%MZ`): it takes priority over the OAuth
/// login, so say how to replace or clear it.
pub fn expired_key_message(
    grant: &AccessGrant,
    expired: &str,
    path: &str,
    oauth_valid: bool,
) -> String {
    let mut lines = vec![format!(
        "the saved API key for this project expired {expired}, and it takes priority over the \
         OAuth login here."
    )];
    lines.extend(crate::cloud::saved_key::recovery_lines(
        path,
        Some(&grant.roles_phrase()),
        oauth_valid,
    ));
    lines.join("\n")
}

/// Refuse, before any request, a command authenticated with the saved key
/// once its `cloud access request` grant has expired: the API would answer a
/// bare 401. A saved key without a grant or an expiry is not refused here.
pub fn refuse_expired_saved_key(source: AuthSource) -> Result<(), CloudError> {
    let Some(grant) = active_grant(source) else {
        return Ok(());
    };
    let Some(expired) = status_expired(&grant, Timestamp::now()) else {
        return Ok(());
    };
    Err(CloudError::auth(expired_key_message(
        &grant,
        &expired,
        &crate::cloud::saved_key::credentials_path_display(),
        crate::cloud::saved_key::oauth_login_valid(),
    )))
}

/// `it` / `them` for a count.
fn pronoun(count: usize) -> &'static str {
    if count == 1 { "it" } else { "them" }
}

/// The message for a bare `FORBIDDEN` from the Cloud API under an API key,
/// for the running command `path` (without the `cloud ` prefix).
pub fn forbidden_message(
    path: &str,
    needs: &DeclaredNeeds,
    grant: Option<&AccessGrant>,
    now: Timestamp,
) -> String {
    let command = format!("`cloud {path}`");
    let mut lines = vec!["FORBIDDEN (the Cloud API gives no reason).".to_string()];
    let Some(grant) = grant else {
        lines.push(if needs.is_empty() {
            format!(
                "  {command} declares no named permission; this key may lack an undeclared one."
            )
        } else {
            format!("  {command} needs: {}", needs.render())
        });
        lines.push(FIX_LINE.into());
        return lines.join("\n");
    };
    let mut key = format!(
        "  Key: from `cloud access request`, {}",
        grant.roles_phrase()
    );
    if let Some(expires) = grant.expires() {
        key.push_str(&format!(", {}", expiry_phrase(expires, now)));
    }
    key.push('.');
    lines.push(key);
    let (subject, lacks, grants) = if grant.roles.len() == 1 {
        ("this role", "lacks", "grants")
    } else {
        ("these roles", "lack", "grant")
    };
    let declared = needs.permissions();
    if declared.is_empty() {
        lines.push(format!(
            "  {command} declares no named permission; {subject} may lack an undeclared one."
        ));
        lines.push(FIX_LINE.into());
        return lines.join("\n");
    }
    let verdicts: Vec<_> = declared
        .iter()
        .map(|permission| (*permission, grant.allows(permission)))
        .collect();
    let lacking: Vec<_> = verdicts
        .iter()
        .filter(|(_, allowed)| *allowed == Some(false))
        .map(|(permission, _)| *permission)
        .collect();
    if !lacking.is_empty() {
        lines.push(format!(
            "  {command} needs {}; {subject} {lacks} {}.",
            lacking.join(", "),
            pronoun(lacking.len())
        ));
        lines.push(FIX_LINE.into());
    } else if verdicts.iter().all(|(_, allowed)| *allowed == Some(true)) {
        lines.push(format!(
            "  {command} needs {}; {subject} {grants} {}, so the refusal has another cause \
             (resource scope, quota, precondition).",
            declared.join(", "),
            pronoun(declared.len())
        ));
    } else {
        // Some role's policies are unknown (the role lookup failed when the
        // key was issued), so what it lacks cannot be told.
        lines.push(format!("  {command} needs: {}", needs.render()));
        lines.push(FIX_LINE.into());
    }
    lines.join("\n")
}

/// Rewrite a bare `FORBIDDEN` (HTTP 403) answered while authenticated with an
/// API key into [`forbidden_message`] for the running command. Every other
/// error, including a 403 whose message carries its own reason, is returned
/// unchanged; the classification and exit code are kept.
pub fn explain_forbidden(client: &CloudClient, error: CloudError) -> CloudError {
    if client.is_bearer_auth()
        || error.failure.and_then(|failure| failure.http_status) != Some(403)
        || error.message.trim() != "FORBIDDEN"
    {
        return error;
    }
    let Some(path) = crate::cloud::permissions::running_command() else {
        return error;
    };
    let Some(needs) = crate::cloud::permissions::declared_needs(path) else {
        return error;
    };
    let grant = active_grant(client.auth_source());
    let message = forbidden_message(path, &needs, grant.as_ref(), Timestamp::now());
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

/// The `clickhouse list` note for an empty list under a saved grant whose roles
/// do not ALLOW `control-plane:service:view`.
pub fn empty_service_list_note(grant: &AccessGrant) -> Option<&'static str> {
    (grant.allows("control-plane:service:view") == Some(false)).then_some(
        "note: this key's role lacks control-plane:service:view; an empty list may mean no access.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(value: &str) -> Timestamp {
        value.parse().unwrap()
    }

    fn role(name: &str, allow: &[&str]) -> GrantedRole {
        GrantedRole {
            id: format!("id-{name}"),
            name: Some(name.into()),
            policies: Some(vec![
                GrantPolicy {
                    effect: "ALLOW".into(),
                    permissions: allow.iter().map(|p| p.to_string()).collect(),
                    resources: vec!["instance/*".into()],
                },
                GrantPolicy {
                    effect: "DENY".into(),
                    permissions: vec!["control-plane:service:delete".into()],
                    resources: vec!["*".into()],
                },
            ]),
        }
    }

    fn grant(roles: Vec<GrantedRole>) -> AccessGrant {
        AccessGrant {
            api_key: "key".into(),
            key_id: "key-id".into(),
            organization_id: "org".into(),
            request_id: "request".into(),
            expires_at: Some("2026-11-19T09:56:10Z".into()),
            roles,
        }
    }

    fn needs(required: &[&'static str], conditional: &[(&str, &'static str)]) -> DeclaredNeeds {
        DeclaredNeeds {
            required: required.to_vec(),
            conditional: conditional
                .iter()
                .map(|(label, permission)| (label.to_string(), *permission))
                .collect(),
        }
    }

    const NOW: &str = "2026-11-19T09:41:00Z";

    #[test]
    fn expiry_phrase_names_the_date_only_off_today_and_says_expired_once_past() {
        let now = at(NOW);
        assert_eq!(
            expiry_phrase(at("2026-11-19T09:56:59Z"), now),
            "expires 09:56Z"
        );
        assert_eq!(
            expiry_phrase(at("2026-11-20T00:05:00Z"), now),
            "expires 2026-11-20T00:05Z"
        );
        assert_eq!(
            expiry_phrase(at("2026-11-19T09:00:00Z"), now),
            "expired 09:00Z"
        );
        assert_eq!(
            expiry_phrase(at("2026-11-18T23:00:00Z"), now),
            "expired 2026-11-18T23:00Z"
        );
    }

    #[test]
    fn status_renders_roles_and_expiry_including_expired() {
        let now = at(NOW);
        let mut saved = grant(vec![
            role("Member", &[]),
            GrantedRole {
                id: "unnamed-id".into(),
                name: None,
                policies: None,
            },
        ]);
        assert_eq!(
            status_scope(&saved, now),
            "Member, unnamed-id, expires 09:56Z"
        );
        assert_eq!(
            status_write_access(&saved, now),
            "API key from cloud access request: Member, unnamed-id; expires 2026-11-19T09:56:10Z"
        );
        assert_eq!(status_expired(&saved, now), None);
        saved.expires_at = Some("2026-11-19T09:00:00Z".into());
        assert_eq!(status_scope(&saved, now), "Member, unnamed-id");
        assert_eq!(
            status_expired(&saved, now).as_deref(),
            Some("2026-11-19T09:00Z")
        );
        assert!(status_write_access(&saved, now).ends_with("; expired 2026-11-19T09:00:00Z"));
        saved.expires_at = None;
        assert_eq!(status_scope(&saved, now), "Member, unnamed-id");
        assert_eq!(status_expired(&saved, now), None);
    }

    #[test]
    fn forbidden_names_what_the_granted_role_lacks() {
        let message = forbidden_message(
            "clickhouse update",
            &needs(
                &["control-plane:service:manage"],
                &[("Removing IPs/endpoints/tags", "control-plane:service:view")],
            ),
            Some(&grant(vec![role(
                "Member",
                &["control-plane:service:view"],
            )])),
            at(NOW),
        );
        let lines: Vec<_> = message.lines().collect();
        assert_eq!(lines[0], "FORBIDDEN (the Cloud API gives no reason).");
        assert_eq!(
            lines[1],
            "  Key: from `cloud access request`, role Member, expires 09:56Z."
        );
        assert_eq!(
            lines[2],
            "  `cloud clickhouse update` needs control-plane:service:manage; this role lacks it."
        );
        assert!(lines[3].starts_with("  Fix: `clickhousectl cloud access roles`"));
        assert_eq!(lines.len(), 4);

        let several = forbidden_message(
            "clickhouse update",
            &needs(
                &["control-plane:service:manage"],
                &[("Removing IPs/endpoints/tags", "control-plane:service:view")],
            ),
            Some(&grant(vec![role("A", &[]), role("B", &[])])),
            at(NOW),
        );
        assert!(several.contains("roles A, B, expires"), "{several}");
        assert!(
            several.contains(
                "needs control-plane:service:manage, control-plane:service:view; these roles lack them."
            ),
            "{several}"
        );
    }

    #[test]
    fn forbidden_with_everything_granted_points_elsewhere_and_drops_the_fix() {
        let message = forbidden_message(
            "clickhouse update",
            &needs(&["control-plane:service:manage"], &[]),
            Some(&grant(vec![role("Admin", &["control-plane:service:*"])])),
            at(NOW),
        );
        assert!(message.contains("Key: from `cloud access request`, role Admin"));
        assert!(message.contains(
            "`cloud clickhouse update` needs control-plane:service:manage; this role grants it, so the refusal has another cause (resource scope, quota, precondition)."
        ), "{message}");
        assert!(!message.contains("Fix:"), "{message}");
    }

    #[test]
    fn forbidden_without_declared_permissions_keeps_the_fix() {
        let message = forbidden_message(
            "clickhouse list",
            &needs(&[], &[]),
            Some(&grant(vec![role("Member", &[])])),
            at(NOW),
        );
        assert!(message.contains(
            "`cloud clickhouse list` declares no named permission; this role may lack an undeclared one."
        ), "{message}");
        assert!(message.contains("Fix:"));
    }

    #[test]
    fn forbidden_without_a_grant_lists_the_declaration() {
        let message = forbidden_message(
            "clickhouse update",
            &needs(
                &["control-plane:service:manage"],
                &[("With --name", "control-plane:service:view")],
            ),
            None,
            at(NOW),
        );
        assert!(!message.contains("Key:"), "{message}");
        assert!(message.contains(
            "`cloud clickhouse update` needs: control-plane:service:manage, (With --name): control-plane:service:view"
        ), "{message}");
        assert!(message.contains("Fix:"));
    }

    #[test]
    fn forbidden_with_unknown_role_policies_does_not_guess() {
        let message = forbidden_message(
            "clickhouse update",
            &needs(&["control-plane:service:manage"], &[]),
            Some(&grant(vec![GrantedRole {
                id: "only-id".into(),
                name: None,
                policies: None,
            }])),
            at(NOW),
        );
        assert!(message.contains("role only-id"), "{message}");
        assert!(!message.contains("lacks"), "{message}");
        assert!(message.contains("needs: control-plane:service:manage"));
        assert!(message.contains("Fix:"));
    }

    #[test]
    fn only_a_bare_forbidden_under_an_api_key_is_rewritten() {
        use crate::failure::{ApiFailure, FailureKind};
        let client = CloudClient::for_tests("http://localhost:1/v1", None);
        let refused = |message: &str| {
            CloudError::new(message)
                .with_failure(ApiFailure::with_status(FailureKind::Http4xx, 403))
        };
        // A 403 with its own reason is kept.
        let kept = explain_forbidden(&client, refused("FORBIDDEN: quota exceeded"));
        assert_eq!(kept.message, "FORBIDDEN: quota exceeded");
        // Other statuses are kept.
        let other = explain_forbidden(
            &client,
            CloudError::new("FORBIDDEN")
                .with_failure(ApiFailure::with_status(FailureKind::Http4xx, 400)),
        );
        assert_eq!(other.message, "FORBIDDEN");
    }

    #[test]
    fn empty_service_list_note_only_when_view_is_known_missing() {
        assert!(empty_service_list_note(&grant(vec![role("Member", &[])])).is_some());
        assert!(
            empty_service_list_note(&grant(vec![role(
                "Viewer",
                &["control-plane:service:view"]
            )]))
            .is_none()
        );
        assert!(
            empty_service_list_note(&grant(vec![GrantedRole {
                id: "x".into(),
                name: None,
                policies: None,
            }]))
            .is_none()
        );
    }

    #[test]
    fn expired_key_message_with_a_valid_oauth_login_offers_a_new_key_or_clearing_it() {
        let message = expired_key_message(
            &grant(vec![role("Service API Reader", &[])]),
            "2026-11-18T17:45Z",
            "/home/dev/work/.clickhouse/credentials.json",
            true,
        );
        assert_eq!(
            message,
            "the saved API key for this project expired 2026-11-18T17:45Z, and it takes priority over the OAuth login here.\n  \
             Key: /home/dev/work/.clickhouse/credentials.json (from `cloud access request`, role Service API Reader)\n  \
             OAuth login: valid (read-only).\n  \
             New key: `clickhousectl cloud access roles`, then `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`\n  \
             Or clear it to use the OAuth login: `clickhousectl cloud auth logout --api-keys`"
        );
    }

    #[test]
    fn expired_key_message_without_a_valid_oauth_login_asks_for_a_login_first() {
        let message = expired_key_message(
            &grant(vec![role("A", &[]), role("B", &[])]),
            "2026-11-18T17:45Z",
            "/home/dev/work/.clickhouse/credentials.json",
            false,
        );
        let lines: Vec<_> = message.lines().collect();
        assert_eq!(lines.len(), 4, "{message}");
        assert_eq!(
            lines[1],
            "  Key: /home/dev/work/.clickhouse/credentials.json (from `cloud access request`, roles A, B)"
        );
        assert_eq!(lines[2], "  OAuth login: none valid.");
        assert_eq!(
            lines[3],
            "  First log in (needs the user): `clickhousectl cloud auth login`. Then: `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`"
        );
    }

    #[test]
    fn a_grant_belongs_only_to_the_key_it_was_saved_with() {
        let mut credentials = crate::cloud::credentials::Credentials {
            api_key: Some("key".into()),
            api_secret: Some("secret".into()),
            access_grant: Some(grant(vec![role("Member", &[])])),
            ..Default::default()
        };
        assert!(grant_of(&credentials).is_some());
        credentials.api_key = Some("other".into());
        assert!(grant_of(&credentials).is_none());
    }
}
