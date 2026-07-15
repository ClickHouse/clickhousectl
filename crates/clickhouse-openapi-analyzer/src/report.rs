use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    MissingClientMethod,
    ExtraClientMethod,
    MissingModelType,
    MissingSchemaDefinition,
    MissingStructField,
    ExtraStructField,
    FieldOptionalityMismatch,
    NewlyBetaOperation,
    GraduatedBetaOperation,
    NewlyDeprecatedField,
    UndeprecatedField,
    MissingDeprecatedMarker,
    StrayDeprecatedMarker,
    MissingEnumValue,
    ExtraEnumValue,
    SnapshotAddedOperation,
    SnapshotRemovedOperation,
    SnapshotAddedSchema,
    SnapshotRemovedSchema,
    StaleExemption,
    UnsupportedEnumConstraint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_pointer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_item: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub details: BTreeMap<String, String>,
}

impl Finding {
    pub(crate) fn new(kind: FindingKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            spec_pointer: None,
            rust_item: None,
            details: BTreeMap::new(),
        }
    }

    pub(crate) fn at_spec(mut self, pointer: impl Into<String>) -> Self {
        self.spec_pointer = Some(pointer.into());
        self
    }

    pub(crate) fn at_rust(mut self, item: impl Into<String>) -> Self {
        self.rust_item = Some(item.into());
        self
    }

    pub(crate) fn detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UnsupportedEnumConstraint {
    pub spec_pointer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_item: Option<String>,
    pub reason: String,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftReport {
    pub schema_version: u32,
    pub findings: Vec<Finding>,
    pub unsupported_enum_constraints: Vec<UnsupportedEnumConstraint>,
}

impl Default for DriftReport {
    fn default() -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            findings: Vec::new(),
            unsupported_enum_constraints: Vec::new(),
        }
    }
}

impl DriftReport {
    pub fn has_drift(&self) -> bool {
        !self.findings.is_empty()
    }

    pub fn actionable_count(&self) -> usize {
        self.findings.len()
    }

    pub fn render_text(&self) -> String {
        if !self.has_drift() {
            return format!(
                "no actionable OpenAPI drift ({} acknowledged unsupported enum constraints)",
                self.unsupported_enum_constraints.len()
            );
        }

        let mut lines = vec![format!(
            "{} actionable OpenAPI drift finding(s):",
            self.actionable_count()
        )];
        for finding in &self.findings {
            let mut locations = Vec::new();
            if let Some(pointer) = &finding.spec_pointer {
                locations.push(pointer.as_str());
            }
            if let Some(item) = &finding.rust_item {
                locations.push(item.as_str());
            }
            let location = if locations.is_empty() {
                String::new()
            } else {
                format!(" [{}]", locations.join(" | "))
            };
            lines.push(format!(
                "- {:?}{}: {}",
                finding.kind, location, finding.message
            ));
        }
        lines.join("\n")
    }

    pub(crate) fn finish(&mut self) {
        self.findings.sort();
        self.findings.dedup();
        self.unsupported_enum_constraints.sort();
        self.unsupported_enum_constraints.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_json_round_trips_and_sorts() {
        let mut report = DriftReport::default();
        report
            .findings
            .push(Finding::new(FindingKind::MissingModelType, "z"));
        report
            .findings
            .push(Finding::new(FindingKind::MissingClientMethod, "a"));
        report.finish();

        let json = serde_json::to_string(&report).unwrap();
        let decoded: DriftReport = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, report);
        assert_eq!(report.findings[0].kind, FindingKind::MissingClientMethod);
    }
}
