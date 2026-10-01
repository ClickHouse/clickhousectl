//! CLI workflows declare their OpenAPI calls beside their domain commands.
//!
//! The shared clap factory refuses incomplete/stale declarations, including
//! commands which run without a required subcommand. This checks declarations,
//! not the Rust call graph: changes to handlers still require call-path review.

use clap::Command;
use clickhouse_cloud_api::meta::{OperationMetadata, operations};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Operations = &'static [&'static OperationMetadata];

#[derive(Clone, Copy, Debug)]
pub(super) struct Conditional {
    pub condition: &'static str,
    pub operations: Operations,
    flag: Option<&'static str>,
}

impl Conditional {
    pub const fn new(condition: &'static str, operations: Operations) -> Self {
        Self {
            condition,
            operations,
            flag: None,
        }
    }

    /// Bind an optional call to an actual flag on the owning command.
    pub const fn flag(flag: &'static str, operations: Operations) -> Self {
        Self {
            condition: flag,
            operations,
            flag: Some(flag),
        }
    }

    fn label(&self) -> String {
        self.flag.map_or_else(
            || self.condition.to_owned(),
            |flag| format!("With --{flag}"),
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Declaration {
    pub path: &'static str,
    pub required: Operations,
    pub conditional: &'static [Conditional],
    pub authorization: Option<&'static str>,
    /// Organization resolution is lazy and adds a list call without --org-id.
    pub organization: bool,
}

impl Declaration {
    pub const fn api(path: &'static str, required: Operations) -> Self {
        Self {
            path,
            required,
            conditional: &[],
            authorization: None,
            organization: true,
        }
    }

    pub const fn non_api(path: &'static str, purpose: &'static str) -> Self {
        Self {
            path,
            required: &[],
            conditional: &[],
            authorization: Some(purpose),
            organization: false,
        }
    }

    pub const fn when(mut self, conditional: &'static [Conditional]) -> Self {
        self.conditional = conditional;
        self
    }

    pub const fn authorization(mut self, authorization: &'static str) -> Self {
        self.authorization = Some(authorization);
        self
    }

    pub const fn unscoped(mut self) -> Self {
        self.organization = false;
        self
    }

    fn groups(&self) -> impl Iterator<Item = Conditional> + '_ {
        self.conditional
            .iter()
            .copied()
            .chain(self.organization.then_some(Conditional::new(
                "Without --org-id",
                &[&operations::ORGANIZATION_GET_LIST],
            )))
    }
}

fn declarations() -> impl Iterator<Item = &'static Declaration> {
    [
        super::activity::PERMISSIONS,
        super::api_keys::PERMISSIONS,
        super::auth::PERMISSIONS,
        super::backups::PERMISSIONS,
        super::clickpipe_endpoints::PERMISSIONS,
        super::clickpipes::PERMISSIONS,
        super::clickstack::PERMISSIONS,
        super::organizations::PERMISSIONS,
        super::postgres::PERMISSIONS,
        super::query_api_endpoints::PERMISSIONS,
        super::services::PERMISSIONS,
        super::udfs::PERMISSIONS,
    ]
    .into_iter()
    .flatten()
}

fn permission_set(operations: Operations) -> BTreeSet<&'static str> {
    operations
        .iter()
        .flat_map(|op| op.required_permissions.iter().copied())
        .collect()
}

/// The renderer only prints additional permissions in conditional groups.
/// Operations remain declared even when they require no named permissions.
fn permission_lines(declaration: &Declaration) -> Vec<String> {
    let required = permission_set(declaration.required);
    let mut lines: Vec<_> = required
        .iter()
        .map(|permission| format!("  API key permission: {permission}"))
        .collect();
    if required.is_empty() && !declaration.required.is_empty() {
        lines.push("  API key: valid key; no additional named permissions declared.".into());
    }
    let mut conditional = BTreeMap::<_, Vec<_>>::new();
    for group in declaration.groups() {
        for permission in permission_set(group.operations).difference(&required) {
            let conditions = conditional.entry(*permission).or_default();
            let label = group.label();
            if !conditions.contains(&label) {
                conditions.push(label);
            }
        }
    }
    for (permission, conditions) in conditional {
        lines.push(format!(
            "  API key ({}): {permission}",
            conditions.join(" or ")
        ));
    }
    if let Some(authorization) = declaration.authorization {
        lines.push(format!("  Authorization: {authorization}"));
    }
    lines
}

fn validate_declaration(declaration: &Declaration) -> Result<(), String> {
    if declaration.path.trim().is_empty() {
        return Err("empty permission declaration path".into());
    }
    if declaration
        .authorization
        .is_some_and(|reason| reason.trim().is_empty())
    {
        return Err(format!(
            "{}: empty authorization explanation",
            declaration.path
        ));
    }
    if declaration.required.is_empty()
        && declaration.conditional.is_empty()
        && declaration.authorization.is_none()
    {
        return Err(format!(
            "{}: declare operations or explain non-OpenAPI authorization",
            declaration.path
        ));
    }
    for group in declaration.groups() {
        if group.condition.trim().is_empty() || group.operations.is_empty() {
            return Err(format!(
                "{}: empty conditional operation group",
                declaration.path
            ));
        }
    }
    for operation in declaration.required.iter().copied().chain(
        declaration
            .groups()
            .flat_map(|group| group.operations.iter().copied()),
    ) {
        let Some(known) = operations::by_operation_id(operation.operation_id) else {
            return Err(format!(
                "{}: unknown operation {}",
                declaration.path, operation.operation_id
            ));
        };
        if known != operation {
            return Err(format!(
                "{}: stale operation {}",
                declaration.path, operation.operation_id
            ));
        }
    }
    Ok(())
}

fn decorate_tree(
    command: &mut Command,
    path: &str,
    declarations: &mut BTreeMap<&str, &Declaration>,
) -> Result<(), String> {
    // clap's Subcommand derive marks pure grouping commands as requiring a
    // subcommand. Optional-subcommand commands can execute their own handler.
    let executable = !path.is_empty() && !command.is_subcommand_required_set();
    if executable {
        let declaration = declarations
            .remove(path)
            .ok_or_else(|| format!("cloud {path}: missing permission declaration"))?;
        for group in declaration.conditional {
            if let Some(flag) = group.flag
                && !command
                    .get_arguments()
                    .any(|argument| argument.get_long() == Some(flag))
            {
                return Err(format!(
                    "cloud {path}: conditional permission references missing --{flag}"
                ));
            }
        }
        let mut context = command
            .get_after_help()
            .map(ToString::to_string)
            .unwrap_or_else(|| "CONTEXT FOR AGENTS:".into());
        for line in permission_lines(declaration) {
            context.push('\n');
            context.push_str(&line);
        }
        *command = std::mem::take(command).after_help(context);
    }
    for child in command.get_subcommands_mut() {
        let child_path = if path.is_empty() {
            child.get_name().to_owned()
        } else {
            format!("{path} {}", child.get_name())
        };
        decorate_tree(child, &child_path, declarations)?;
    }
    Ok(())
}

fn decorate_with<'a>(
    mut command: Command,
    bindings: impl IntoIterator<Item = &'a Declaration>,
) -> Result<Command, String> {
    let mut registry = BTreeMap::new();
    for declaration in bindings {
        validate_declaration(declaration)?;
        if registry.insert(declaration.path, declaration).is_some() {
            return Err(format!(
                "{}: duplicate permission declaration",
                declaration.path
            ));
        }
    }
    let cloud = command
        .find_subcommand_mut("cloud")
        .ok_or("missing cloud command")?;
    decorate_tree(cloud, "", &mut registry)?;
    if !registry.is_empty() {
        return Err(format!(
            "permission declarations without an executable command: {}",
            registry.keys().copied().collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(command)
}

pub(crate) fn decorate(command: Command) -> Command {
    decorate_with(command, declarations())
        .expect("every cloud command must declare its permission requirements")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::{Args, CommandFactory, Parser};

    fn raw_command() -> Command {
        <Cli as Args>::augment_args(Command::new("clickhousectl"))
    }

    #[test]
    fn every_executable_cloud_command_has_exactly_one_valid_declaration() {
        decorate_with(raw_command(), declarations()).unwrap();
        decorate_with(
            <Cli as Args>::augment_args_for_update(Command::new("clickhousectl")),
            declarations(),
        )
        .unwrap();
    }

    #[test]
    fn undeclared_new_commands_and_stale_duplicate_or_empty_declarations_fail_closed() {
        let mut command = raw_command();
        *command.find_subcommand_mut("cloud").unwrap() = command
            .find_subcommand("cloud")
            .unwrap()
            .clone()
            .subcommand(Command::new("new-command"));
        assert!(
            decorate_with(command, declarations())
                .unwrap_err()
                .contains("new-command")
        );
        const STALE: Declaration =
            Declaration::api("removed-command", &[&operations::INSTANCE_GET]);
        assert!(
            decorate_with(raw_command(), declarations().chain([&STALE]))
                .unwrap_err()
                .contains("removed-command")
        );
        let duplicate = declarations().next().unwrap();
        assert!(
            decorate_with(raw_command(), declarations().chain([duplicate]))
                .unwrap_err()
                .contains("duplicate")
        );
        assert!(validate_declaration(&Declaration::api("empty", &[])).is_err());
        assert!(validate_declaration(&Declaration::non_api("empty", " ")).is_err());
    }

    #[test]
    fn permission_union_is_sorted_and_deduplicated_and_conditions_keep_their_scope() {
        const DECLARATION: Declaration = Declaration::api(
            "sample",
            &[&operations::INSTANCE_GET, &operations::INSTANCE_GET],
        )
        .when(&[Conditional::new(
            "With --force",
            &[&operations::INSTANCE_GET, &operations::INSTANCE_DELETE],
        )])
        .unscoped();
        assert_eq!(
            permission_set(DECLARATION.required),
            operations::INSTANCE_GET
                .required_permissions
                .iter()
                .copied()
                .collect()
        );
        let required = permission_set(DECLARATION.required);
        let additional: BTreeSet<_> = permission_set(DECLARATION.conditional[0].operations)
            .difference(&required)
            .copied()
            .collect();
        let lines = permission_lines(&DECLARATION);
        assert_eq!(lines.len(), required.len() + additional.len());
        for permission in additional {
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains(DECLARATION.conditional[0].condition)
                        && line.ends_with(permission))
            );
        }
    }

    #[test]
    fn one_permission_retains_every_alternative_condition() {
        const DECLARATION: Declaration = Declaration::api("sample", &[])
            .when(&[
                Conditional::new("setup", &[&operations::INSTANCE_GET]),
                Conditional::new("diagnosis", &[&operations::INSTANCE_GET]),
                Conditional::new("setup", &[&operations::INSTANCE_GET]),
            ])
            .unscoped();
        let lines = permission_lines(&DECLARATION);
        assert_eq!(
            lines.len(),
            operations::INSTANCE_GET.required_permissions.len()
        );
        for permission in operations::INSTANCE_GET.required_permissions {
            let line = lines
                .iter()
                .find(|line| line.ends_with(permission))
                .unwrap();
            assert_eq!(line.matches("setup").count(), 1);
            assert_eq!(line.matches("diagnosis").count(), 1);
        }
    }

    #[test]
    fn flag_conditions_and_operation_catalog_references_are_validated() {
        const MISSING_FLAG: Declaration =
            Declaration::api("service get", &[&operations::INSTANCE_GET]).when(&[
                Conditional::flag("missing", &[&operations::INSTANCE_GET_LIST]),
            ]);
        let registry = declarations()
            .filter(|declaration| declaration.path != "service get")
            .chain([&MISSING_FLAG]);
        assert!(
            decorate_with(raw_command(), registry)
                .unwrap_err()
                .contains("--missing")
        );
        const UNKNOWN: OperationMetadata = {
            let mut metadata = operations::INSTANCE_GET;
            metadata.operation_id = "unknown";
            metadata
        };
        const STALE: OperationMetadata = {
            let mut metadata = operations::INSTANCE_GET;
            metadata.required_permissions = &[];
            metadata
        };
        assert!(validate_declaration(&Declaration::api("sample", &[&UNKNOWN])).is_err());
        assert!(validate_declaration(&Declaration::api("sample", &[&STALE])).is_err());
    }

    #[test]
    fn decoration_preserves_context_and_leaves_pure_groups_and_local_help_unchanged() {
        fn compare(before: &Command, after: &Command, path: &str) {
            let declared = declarations().any(|item| format!("cloud {}", item.path) == path);
            let original = before.get_after_help().map(ToString::to_string);
            let decorated = after.get_after_help().map(ToString::to_string);
            if declared {
                if let Some(original) = original {
                    assert!(decorated.as_ref().unwrap().starts_with(&original), "{path}");
                }
            } else {
                assert_eq!(original, decorated, "{path}");
            }
            for child in before.get_subcommands() {
                let next = if path.is_empty() {
                    child.get_name().to_owned()
                } else {
                    format!("{path} {}", child.get_name())
                };
                compare(
                    child,
                    after.find_subcommand(child.get_name()).unwrap(),
                    &next,
                );
            }
        }
        compare(&raw_command(), &Cli::command(), "");
    }

    #[test]
    fn parser_and_command_factory_use_the_same_permission_help() {
        let error = Cli::try_parse_from(["chctl", "cloud", "service", "get", "--help"])
            .err()
            .unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let mut command = Cli::command();
        command.build();
        let leaf = command
            .find_subcommand_mut("cloud")
            .unwrap()
            .find_subcommand_mut("service")
            .unwrap()
            .find_subcommand_mut("get")
            .unwrap();
        for permission in operations::INSTANCE_GET.required_permissions {
            assert!(error.to_string().contains(permission));
            assert!(leaf.render_help().to_string().contains(permission));
            assert!(leaf.render_long_help().to_string().contains(permission));
        }
    }

    #[test]
    fn inherited_empty_requirements_are_distinct_from_non_api_commands() {
        const EMPTY: OperationMetadata = {
            let mut metadata = operations::ORGANIZATION_GET_LIST;
            metadata.required_permissions = &[];
            metadata
        };
        let api = Declaration::api("org list", &[&EMPTY]).unscoped();
        let local = Declaration::non_api(
            "auth logout",
            "Local credential removal; no Cloud API call.",
        );
        assert!(api.authorization.is_none());
        assert!(!api.required.is_empty());
        assert!(local.required.is_empty());
        assert!(local.authorization.is_some());
        assert!(!permission_lines(&api).is_empty());
        assert!(!permission_lines(&local).is_empty());
    }
}
