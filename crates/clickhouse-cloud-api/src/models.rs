//! Typed models for ClickHouse Cloud API schemas.
//!
//! Auto-generated from the OpenAPI specification.

use serde::{Deserialize, Serialize};

/// `pgHaType` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgHaType {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "async")]
    Async,
    #[serde(rename = "sync")]
    Sync,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgHaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Async => write!(f, "async"),
            Self::Sync => write!(f, "sync"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `pgProvider` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `pgSize` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgSize {
    #[serde(rename = "c6gd.medium")]
    #[default]
    C6gd_medium,
    #[serde(rename = "c6gd.large")]
    C6gd_large,
    #[serde(rename = "c6gd.xlarge")]
    C6gd_xlarge,
    #[serde(rename = "c6gd.2xlarge")]
    C6gd_2xlarge,
    #[serde(rename = "c6gd.4xlarge")]
    C6gd_4xlarge,
    #[serde(rename = "c6gd.8xlarge")]
    C6gd_8xlarge,
    #[serde(rename = "c6gd.12xlarge")]
    C6gd_12xlarge,
    #[serde(rename = "c6gd.16xlarge")]
    C6gd_16xlarge,
    #[serde(rename = "c6gd.metal")]
    C6gd_metal,
    #[serde(rename = "i7i.large")]
    I7i_large,
    #[serde(rename = "i7i.xlarge")]
    I7i_xlarge,
    #[serde(rename = "i7i.2xlarge")]
    I7i_2xlarge,
    #[serde(rename = "i7i.4xlarge")]
    I7i_4xlarge,
    #[serde(rename = "i7i.8xlarge")]
    I7i_8xlarge,
    #[serde(rename = "i7i.12xlarge")]
    I7i_12xlarge,
    #[serde(rename = "i7i.16xlarge")]
    I7i_16xlarge,
    #[serde(rename = "i7i.24xlarge")]
    I7i_24xlarge,
    #[serde(rename = "i7i.metal-24xl")]
    I7i_metal_24xl,
    #[serde(rename = "i7i.48xlarge")]
    I7i_48xlarge,
    #[serde(rename = "i7i.metal-48xl")]
    I7i_metal_48xl,
    #[serde(rename = "i7ie.large")]
    I7ie_large,
    #[serde(rename = "i7ie.xlarge")]
    I7ie_xlarge,
    #[serde(rename = "i7ie.2xlarge")]
    I7ie_2xlarge,
    #[serde(rename = "i7ie.3xlarge")]
    I7ie_3xlarge,
    #[serde(rename = "i7ie.6xlarge")]
    I7ie_6xlarge,
    #[serde(rename = "i7ie.12xlarge")]
    I7ie_12xlarge,
    #[serde(rename = "i7ie.18xlarge")]
    I7ie_18xlarge,
    #[serde(rename = "i7ie.24xlarge")]
    I7ie_24xlarge,
    #[serde(rename = "i7ie.metal-24xl")]
    I7ie_metal_24xl,
    #[serde(rename = "i7ie.48xlarge")]
    I7ie_48xlarge,
    #[serde(rename = "i7ie.metal-48xl")]
    I7ie_metal_48xl,
    #[serde(rename = "i8g.large")]
    I8g_large,
    #[serde(rename = "i8g.xlarge")]
    I8g_xlarge,
    #[serde(rename = "i8g.2xlarge")]
    I8g_2xlarge,
    #[serde(rename = "i8g.4xlarge")]
    I8g_4xlarge,
    #[serde(rename = "i8g.8xlarge")]
    I8g_8xlarge,
    #[serde(rename = "i8g.12xlarge")]
    I8g_12xlarge,
    #[serde(rename = "i8g.16xlarge")]
    I8g_16xlarge,
    #[serde(rename = "i8g.24xlarge")]
    I8g_24xlarge,
    #[serde(rename = "i8g.metal-24xl")]
    I8g_metal_24xl,
    #[serde(rename = "i8g.48xlarge")]
    I8g_48xlarge,
    #[serde(rename = "i8ge.large")]
    I8ge_large,
    #[serde(rename = "i8ge.xlarge")]
    I8ge_xlarge,
    #[serde(rename = "i8ge.2xlarge")]
    I8ge_2xlarge,
    #[serde(rename = "i8ge.3xlarge")]
    I8ge_3xlarge,
    #[serde(rename = "i8ge.6xlarge")]
    I8ge_6xlarge,
    #[serde(rename = "i8ge.12xlarge")]
    I8ge_12xlarge,
    #[serde(rename = "i8ge.18xlarge")]
    I8ge_18xlarge,
    #[serde(rename = "i8ge.24xlarge")]
    I8ge_24xlarge,
    #[serde(rename = "i8ge.metal-24xl")]
    I8ge_metal_24xl,
    #[serde(rename = "i8ge.48xlarge")]
    I8ge_48xlarge,
    #[serde(rename = "i8ge.metal-48xl")]
    I8ge_metal_48xl,
    #[serde(rename = "m6a.large")]
    M6a_large,
    #[serde(rename = "m6a.xlarge")]
    M6a_xlarge,
    #[serde(rename = "m6a.2xlarge")]
    M6a_2xlarge,
    #[serde(rename = "m6a.4xlarge")]
    M6a_4xlarge,
    #[serde(rename = "m6a.8xlarge")]
    M6a_8xlarge,
    #[serde(rename = "m6a.12xlarge")]
    M6a_12xlarge,
    #[serde(rename = "m6a.16xlarge")]
    M6a_16xlarge,
    #[serde(rename = "m6a.24xlarge")]
    M6a_24xlarge,
    #[serde(rename = "m6a.32xlarge")]
    M6a_32xlarge,
    #[serde(rename = "m6a.48xlarge")]
    M6a_48xlarge,
    #[serde(rename = "m6a.metal")]
    M6a_metal,
    #[serde(rename = "m6gd.medium")]
    M6gd_medium,
    #[serde(rename = "m6gd.large")]
    M6gd_large,
    #[serde(rename = "m6gd.xlarge")]
    M6gd_xlarge,
    #[serde(rename = "m6gd.2xlarge")]
    M6gd_2xlarge,
    #[serde(rename = "m6gd.4xlarge")]
    M6gd_4xlarge,
    #[serde(rename = "m6gd.8xlarge")]
    M6gd_8xlarge,
    #[serde(rename = "m6gd.12xlarge")]
    M6gd_12xlarge,
    #[serde(rename = "m6gd.16xlarge")]
    M6gd_16xlarge,
    #[serde(rename = "m6gd.metal")]
    M6gd_metal,
    #[serde(rename = "m6id.large")]
    M6id_large,
    #[serde(rename = "m6id.xlarge")]
    M6id_xlarge,
    #[serde(rename = "m6id.2xlarge")]
    M6id_2xlarge,
    #[serde(rename = "m6id.4xlarge")]
    M6id_4xlarge,
    #[serde(rename = "m6id.8xlarge")]
    M6id_8xlarge,
    #[serde(rename = "m6id.12xlarge")]
    M6id_12xlarge,
    #[serde(rename = "m6id.16xlarge")]
    M6id_16xlarge,
    #[serde(rename = "m6id.24xlarge")]
    M6id_24xlarge,
    #[serde(rename = "m6id.32xlarge")]
    M6id_32xlarge,
    #[serde(rename = "m6id.metal")]
    M6id_metal,
    #[serde(rename = "m7a.medium")]
    M7a_medium,
    #[serde(rename = "m7a.large")]
    M7a_large,
    #[serde(rename = "m7a.xlarge")]
    M7a_xlarge,
    #[serde(rename = "m7a.2xlarge")]
    M7a_2xlarge,
    #[serde(rename = "m7a.4xlarge")]
    M7a_4xlarge,
    #[serde(rename = "m7a.8xlarge")]
    M7a_8xlarge,
    #[serde(rename = "m7a.12xlarge")]
    M7a_12xlarge,
    #[serde(rename = "m7a.16xlarge")]
    M7a_16xlarge,
    #[serde(rename = "m7a.24xlarge")]
    M7a_24xlarge,
    #[serde(rename = "m7a.32xlarge")]
    M7a_32xlarge,
    #[serde(rename = "m7a.48xlarge")]
    M7a_48xlarge,
    #[serde(rename = "m7a.metal-48xl")]
    M7a_metal_48xl,
    #[serde(rename = "m7i.large")]
    M7i_large,
    #[serde(rename = "m7i.xlarge")]
    M7i_xlarge,
    #[serde(rename = "m7i.2xlarge")]
    M7i_2xlarge,
    #[serde(rename = "m7i.4xlarge")]
    M7i_4xlarge,
    #[serde(rename = "m7i.8xlarge")]
    M7i_8xlarge,
    #[serde(rename = "m7i.12xlarge")]
    M7i_12xlarge,
    #[serde(rename = "m7i.16xlarge")]
    M7i_16xlarge,
    #[serde(rename = "m7i.24xlarge")]
    M7i_24xlarge,
    #[serde(rename = "m7i.metal-24xl")]
    M7i_metal_24xl,
    #[serde(rename = "m7i.48xlarge")]
    M7i_48xlarge,
    #[serde(rename = "m7i.metal-48xl")]
    M7i_metal_48xl,
    #[serde(rename = "m8gd.medium")]
    M8gd_medium,
    #[serde(rename = "m8gd.large")]
    M8gd_large,
    #[serde(rename = "m8gd.xlarge")]
    M8gd_xlarge,
    #[serde(rename = "m8gd.2xlarge")]
    M8gd_2xlarge,
    #[serde(rename = "m8gd.4xlarge")]
    M8gd_4xlarge,
    #[serde(rename = "m8gd.8xlarge")]
    M8gd_8xlarge,
    #[serde(rename = "m8gd.12xlarge")]
    M8gd_12xlarge,
    #[serde(rename = "m8gd.16xlarge")]
    M8gd_16xlarge,
    #[serde(rename = "m8gd.24xlarge")]
    M8gd_24xlarge,
    #[serde(rename = "m8gd.metal-24xl")]
    M8gd_metal_24xl,
    #[serde(rename = "m8gd.48xlarge")]
    M8gd_48xlarge,
    #[serde(rename = "m8gd.metal-48xl")]
    M8gd_metal_48xl,
    #[serde(rename = "r6gd.medium")]
    R6gd_medium,
    #[serde(rename = "r6gd.large")]
    R6gd_large,
    #[serde(rename = "r6gd.xlarge")]
    R6gd_xlarge,
    #[serde(rename = "r6gd.2xlarge")]
    R6gd_2xlarge,
    #[serde(rename = "r6gd.4xlarge")]
    R6gd_4xlarge,
    #[serde(rename = "r6gd.8xlarge")]
    R6gd_8xlarge,
    #[serde(rename = "r6gd.12xlarge")]
    R6gd_12xlarge,
    #[serde(rename = "r6gd.16xlarge")]
    R6gd_16xlarge,
    #[serde(rename = "r6gd.metal")]
    R6gd_metal,
    #[serde(rename = "r6id.large")]
    R6id_large,
    #[serde(rename = "r6id.xlarge")]
    R6id_xlarge,
    #[serde(rename = "r6id.2xlarge")]
    R6id_2xlarge,
    #[serde(rename = "r6id.4xlarge")]
    R6id_4xlarge,
    #[serde(rename = "r6id.8xlarge")]
    R6id_8xlarge,
    #[serde(rename = "r6id.12xlarge")]
    R6id_12xlarge,
    #[serde(rename = "r6id.16xlarge")]
    R6id_16xlarge,
    #[serde(rename = "r6id.24xlarge")]
    R6id_24xlarge,
    #[serde(rename = "r6id.32xlarge")]
    R6id_32xlarge,
    #[serde(rename = "r6id.metal")]
    R6id_metal,
    #[serde(rename = "r8gd.medium")]
    R8gd_medium,
    #[serde(rename = "r8gd.large")]
    R8gd_large,
    #[serde(rename = "r8gd.xlarge")]
    R8gd_xlarge,
    #[serde(rename = "r8gd.2xlarge")]
    R8gd_2xlarge,
    #[serde(rename = "r8gd.4xlarge")]
    R8gd_4xlarge,
    #[serde(rename = "r8gd.8xlarge")]
    R8gd_8xlarge,
    #[serde(rename = "r8gd.12xlarge")]
    R8gd_12xlarge,
    #[serde(rename = "r8gd.16xlarge")]
    R8gd_16xlarge,
    #[serde(rename = "r8gd.24xlarge")]
    R8gd_24xlarge,
    #[serde(rename = "r8gd.metal-24xl")]
    R8gd_metal_24xl,
    #[serde(rename = "r8gd.48xlarge")]
    R8gd_48xlarge,
    #[serde(rename = "r8gd.metal-48xl")]
    R8gd_metal_48xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::C6gd_medium => write!(f, "c6gd.medium"),
            Self::C6gd_large => write!(f, "c6gd.large"),
            Self::C6gd_xlarge => write!(f, "c6gd.xlarge"),
            Self::C6gd_2xlarge => write!(f, "c6gd.2xlarge"),
            Self::C6gd_4xlarge => write!(f, "c6gd.4xlarge"),
            Self::C6gd_8xlarge => write!(f, "c6gd.8xlarge"),
            Self::C6gd_12xlarge => write!(f, "c6gd.12xlarge"),
            Self::C6gd_16xlarge => write!(f, "c6gd.16xlarge"),
            Self::C6gd_metal => write!(f, "c6gd.metal"),
            Self::I7i_large => write!(f, "i7i.large"),
            Self::I7i_xlarge => write!(f, "i7i.xlarge"),
            Self::I7i_2xlarge => write!(f, "i7i.2xlarge"),
            Self::I7i_4xlarge => write!(f, "i7i.4xlarge"),
            Self::I7i_8xlarge => write!(f, "i7i.8xlarge"),
            Self::I7i_12xlarge => write!(f, "i7i.12xlarge"),
            Self::I7i_16xlarge => write!(f, "i7i.16xlarge"),
            Self::I7i_24xlarge => write!(f, "i7i.24xlarge"),
            Self::I7i_metal_24xl => write!(f, "i7i.metal-24xl"),
            Self::I7i_48xlarge => write!(f, "i7i.48xlarge"),
            Self::I7i_metal_48xl => write!(f, "i7i.metal-48xl"),
            Self::I7ie_large => write!(f, "i7ie.large"),
            Self::I7ie_xlarge => write!(f, "i7ie.xlarge"),
            Self::I7ie_2xlarge => write!(f, "i7ie.2xlarge"),
            Self::I7ie_3xlarge => write!(f, "i7ie.3xlarge"),
            Self::I7ie_6xlarge => write!(f, "i7ie.6xlarge"),
            Self::I7ie_12xlarge => write!(f, "i7ie.12xlarge"),
            Self::I7ie_18xlarge => write!(f, "i7ie.18xlarge"),
            Self::I7ie_24xlarge => write!(f, "i7ie.24xlarge"),
            Self::I7ie_metal_24xl => write!(f, "i7ie.metal-24xl"),
            Self::I7ie_48xlarge => write!(f, "i7ie.48xlarge"),
            Self::I7ie_metal_48xl => write!(f, "i7ie.metal-48xl"),
            Self::I8g_large => write!(f, "i8g.large"),
            Self::I8g_xlarge => write!(f, "i8g.xlarge"),
            Self::I8g_2xlarge => write!(f, "i8g.2xlarge"),
            Self::I8g_4xlarge => write!(f, "i8g.4xlarge"),
            Self::I8g_8xlarge => write!(f, "i8g.8xlarge"),
            Self::I8g_12xlarge => write!(f, "i8g.12xlarge"),
            Self::I8g_16xlarge => write!(f, "i8g.16xlarge"),
            Self::I8g_24xlarge => write!(f, "i8g.24xlarge"),
            Self::I8g_metal_24xl => write!(f, "i8g.metal-24xl"),
            Self::I8g_48xlarge => write!(f, "i8g.48xlarge"),
            Self::I8ge_large => write!(f, "i8ge.large"),
            Self::I8ge_xlarge => write!(f, "i8ge.xlarge"),
            Self::I8ge_2xlarge => write!(f, "i8ge.2xlarge"),
            Self::I8ge_3xlarge => write!(f, "i8ge.3xlarge"),
            Self::I8ge_6xlarge => write!(f, "i8ge.6xlarge"),
            Self::I8ge_12xlarge => write!(f, "i8ge.12xlarge"),
            Self::I8ge_18xlarge => write!(f, "i8ge.18xlarge"),
            Self::I8ge_24xlarge => write!(f, "i8ge.24xlarge"),
            Self::I8ge_metal_24xl => write!(f, "i8ge.metal-24xl"),
            Self::I8ge_48xlarge => write!(f, "i8ge.48xlarge"),
            Self::I8ge_metal_48xl => write!(f, "i8ge.metal-48xl"),
            Self::M6a_large => write!(f, "m6a.large"),
            Self::M6a_xlarge => write!(f, "m6a.xlarge"),
            Self::M6a_2xlarge => write!(f, "m6a.2xlarge"),
            Self::M6a_4xlarge => write!(f, "m6a.4xlarge"),
            Self::M6a_8xlarge => write!(f, "m6a.8xlarge"),
            Self::M6a_12xlarge => write!(f, "m6a.12xlarge"),
            Self::M6a_16xlarge => write!(f, "m6a.16xlarge"),
            Self::M6a_24xlarge => write!(f, "m6a.24xlarge"),
            Self::M6a_32xlarge => write!(f, "m6a.32xlarge"),
            Self::M6a_48xlarge => write!(f, "m6a.48xlarge"),
            Self::M6a_metal => write!(f, "m6a.metal"),
            Self::M6gd_medium => write!(f, "m6gd.medium"),
            Self::M6gd_large => write!(f, "m6gd.large"),
            Self::M6gd_xlarge => write!(f, "m6gd.xlarge"),
            Self::M6gd_2xlarge => write!(f, "m6gd.2xlarge"),
            Self::M6gd_4xlarge => write!(f, "m6gd.4xlarge"),
            Self::M6gd_8xlarge => write!(f, "m6gd.8xlarge"),
            Self::M6gd_12xlarge => write!(f, "m6gd.12xlarge"),
            Self::M6gd_16xlarge => write!(f, "m6gd.16xlarge"),
            Self::M6gd_metal => write!(f, "m6gd.metal"),
            Self::M6id_large => write!(f, "m6id.large"),
            Self::M6id_xlarge => write!(f, "m6id.xlarge"),
            Self::M6id_2xlarge => write!(f, "m6id.2xlarge"),
            Self::M6id_4xlarge => write!(f, "m6id.4xlarge"),
            Self::M6id_8xlarge => write!(f, "m6id.8xlarge"),
            Self::M6id_12xlarge => write!(f, "m6id.12xlarge"),
            Self::M6id_16xlarge => write!(f, "m6id.16xlarge"),
            Self::M6id_24xlarge => write!(f, "m6id.24xlarge"),
            Self::M6id_32xlarge => write!(f, "m6id.32xlarge"),
            Self::M6id_metal => write!(f, "m6id.metal"),
            Self::M7a_medium => write!(f, "m7a.medium"),
            Self::M7a_large => write!(f, "m7a.large"),
            Self::M7a_xlarge => write!(f, "m7a.xlarge"),
            Self::M7a_2xlarge => write!(f, "m7a.2xlarge"),
            Self::M7a_4xlarge => write!(f, "m7a.4xlarge"),
            Self::M7a_8xlarge => write!(f, "m7a.8xlarge"),
            Self::M7a_12xlarge => write!(f, "m7a.12xlarge"),
            Self::M7a_16xlarge => write!(f, "m7a.16xlarge"),
            Self::M7a_24xlarge => write!(f, "m7a.24xlarge"),
            Self::M7a_32xlarge => write!(f, "m7a.32xlarge"),
            Self::M7a_48xlarge => write!(f, "m7a.48xlarge"),
            Self::M7a_metal_48xl => write!(f, "m7a.metal-48xl"),
            Self::M7i_large => write!(f, "m7i.large"),
            Self::M7i_xlarge => write!(f, "m7i.xlarge"),
            Self::M7i_2xlarge => write!(f, "m7i.2xlarge"),
            Self::M7i_4xlarge => write!(f, "m7i.4xlarge"),
            Self::M7i_8xlarge => write!(f, "m7i.8xlarge"),
            Self::M7i_12xlarge => write!(f, "m7i.12xlarge"),
            Self::M7i_16xlarge => write!(f, "m7i.16xlarge"),
            Self::M7i_24xlarge => write!(f, "m7i.24xlarge"),
            Self::M7i_metal_24xl => write!(f, "m7i.metal-24xl"),
            Self::M7i_48xlarge => write!(f, "m7i.48xlarge"),
            Self::M7i_metal_48xl => write!(f, "m7i.metal-48xl"),
            Self::M8gd_medium => write!(f, "m8gd.medium"),
            Self::M8gd_large => write!(f, "m8gd.large"),
            Self::M8gd_xlarge => write!(f, "m8gd.xlarge"),
            Self::M8gd_2xlarge => write!(f, "m8gd.2xlarge"),
            Self::M8gd_4xlarge => write!(f, "m8gd.4xlarge"),
            Self::M8gd_8xlarge => write!(f, "m8gd.8xlarge"),
            Self::M8gd_12xlarge => write!(f, "m8gd.12xlarge"),
            Self::M8gd_16xlarge => write!(f, "m8gd.16xlarge"),
            Self::M8gd_24xlarge => write!(f, "m8gd.24xlarge"),
            Self::M8gd_metal_24xl => write!(f, "m8gd.metal-24xl"),
            Self::M8gd_48xlarge => write!(f, "m8gd.48xlarge"),
            Self::M8gd_metal_48xl => write!(f, "m8gd.metal-48xl"),
            Self::R6gd_medium => write!(f, "r6gd.medium"),
            Self::R6gd_large => write!(f, "r6gd.large"),
            Self::R6gd_xlarge => write!(f, "r6gd.xlarge"),
            Self::R6gd_2xlarge => write!(f, "r6gd.2xlarge"),
            Self::R6gd_4xlarge => write!(f, "r6gd.4xlarge"),
            Self::R6gd_8xlarge => write!(f, "r6gd.8xlarge"),
            Self::R6gd_12xlarge => write!(f, "r6gd.12xlarge"),
            Self::R6gd_16xlarge => write!(f, "r6gd.16xlarge"),
            Self::R6gd_metal => write!(f, "r6gd.metal"),
            Self::R6id_large => write!(f, "r6id.large"),
            Self::R6id_xlarge => write!(f, "r6id.xlarge"),
            Self::R6id_2xlarge => write!(f, "r6id.2xlarge"),
            Self::R6id_4xlarge => write!(f, "r6id.4xlarge"),
            Self::R6id_8xlarge => write!(f, "r6id.8xlarge"),
            Self::R6id_12xlarge => write!(f, "r6id.12xlarge"),
            Self::R6id_16xlarge => write!(f, "r6id.16xlarge"),
            Self::R6id_24xlarge => write!(f, "r6id.24xlarge"),
            Self::R6id_32xlarge => write!(f, "r6id.32xlarge"),
            Self::R6id_metal => write!(f, "r6id.metal"),
            Self::R8gd_medium => write!(f, "r8gd.medium"),
            Self::R8gd_large => write!(f, "r8gd.large"),
            Self::R8gd_xlarge => write!(f, "r8gd.xlarge"),
            Self::R8gd_2xlarge => write!(f, "r8gd.2xlarge"),
            Self::R8gd_4xlarge => write!(f, "r8gd.4xlarge"),
            Self::R8gd_8xlarge => write!(f, "r8gd.8xlarge"),
            Self::R8gd_12xlarge => write!(f, "r8gd.12xlarge"),
            Self::R8gd_16xlarge => write!(f, "r8gd.16xlarge"),
            Self::R8gd_24xlarge => write!(f, "r8gd.24xlarge"),
            Self::R8gd_metal_24xl => write!(f, "r8gd.metal-24xl"),
            Self::R8gd_48xlarge => write!(f, "r8gd.48xlarge"),
            Self::R8gd_metal_48xl => write!(f, "r8gd.metal-48xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `pgStateProperty` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgStateProperty {
    #[serde(rename = "creating")]
    #[default]
    Creating,
    #[serde(rename = "restarting")]
    Restarting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "replaying_wal")]
    Replaying_wal,
    #[serde(rename = "restoring_backup")]
    Restoring_backup,
    #[serde(rename = "finalizing_restore")]
    Finalizing_restore,
    #[serde(rename = "unavailable")]
    Unavailable,
    #[serde(rename = "deleting")]
    Deleting,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgStateProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Restarting => write!(f, "restarting"),
            Self::Running => write!(f, "running"),
            Self::Replaying_wal => write!(f, "replaying_wal"),
            Self::Restoring_backup => write!(f, "restoring_backup"),
            Self::Finalizing_restore => write!(f, "finalizing_restore"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::Deleting => write!(f, "deleting"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `pgVersion` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgVersion {
    #[serde(rename = "18")]
    #[default]
    _18,
    #[serde(rename = "17")]
    _17,
    #[serde(rename = "16")]
    _16,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_18 => write!(f, "18"),
            Self::_17 => write!(f, "17"),
            Self::_16 => write!(f, "16"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Activity.actorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityActortype {
    #[serde(rename = "user")]
    #[default]
    User,
    #[serde(rename = "support")]
    Support,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "api")]
    Api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityActortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Support => write!(f, "support"),
            Self::System => write!(f, "system"),
            Self::Api => write!(f, "api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Activity.keyUpdateType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityKeyupdatetype {
    #[serde(rename = "created")]
    #[default]
    Created,
    #[serde(rename = "deleted")]
    Deleted,
    #[serde(rename = "name-changed")]
    Name_changed,
    #[serde(rename = "role-changed")]
    Role_changed,
    #[serde(rename = "state-changed")]
    State_changed,
    #[serde(rename = "date-changed")]
    Date_changed,
    #[serde(rename = "ip-access-list-changed")]
    Ip_access_list_changed,
    #[serde(rename = "org-role-changed")]
    Org_role_changed,
    #[serde(rename = "default-service-role-changed")]
    Default_service_role_changed,
    #[serde(rename = "service-role-changed")]
    Service_role_changed,
    #[serde(rename = "roles-v2-changed")]
    Roles_v2_changed,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityKeyupdatetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Deleted => write!(f, "deleted"),
            Self::Name_changed => write!(f, "name-changed"),
            Self::Role_changed => write!(f, "role-changed"),
            Self::State_changed => write!(f, "state-changed"),
            Self::Date_changed => write!(f, "date-changed"),
            Self::Ip_access_list_changed => write!(f, "ip-access-list-changed"),
            Self::Org_role_changed => write!(f, "org-role-changed"),
            Self::Default_service_role_changed => write!(f, "default-service-role-changed"),
            Self::Service_role_changed => write!(f, "service-role-changed"),
            Self::Roles_v2_changed => write!(f, "roles-v2-changed"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Activity.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityType {
    #[serde(rename = "create_organization")]
    #[default]
    Create_organization,
    #[serde(rename = "organization_update_name")]
    Organization_update_name,
    #[serde(rename = "transfer_service_in")]
    Transfer_service_in,
    #[serde(rename = "transfer_service_out")]
    Transfer_service_out,
    #[serde(rename = "save_payment_method")]
    Save_payment_method,
    #[serde(rename = "marketplace_subscription")]
    Marketplace_subscription,
    #[serde(rename = "migrate_marketplace_billing_details_in")]
    Migrate_marketplace_billing_details_in,
    #[serde(rename = "migrate_marketplace_billing_details_out")]
    Migrate_marketplace_billing_details_out,
    #[serde(rename = "organization_update_tier")]
    Organization_update_tier,
    #[serde(rename = "organization_invite_create")]
    Organization_invite_create,
    #[serde(rename = "organization_invite_delete")]
    Organization_invite_delete,
    #[serde(rename = "organization_member_join")]
    Organization_member_join,
    #[serde(rename = "organization_member_add")]
    Organization_member_add,
    #[serde(rename = "organization_member_leave")]
    Organization_member_leave,
    #[serde(rename = "organization_member_delete")]
    Organization_member_delete,
    #[serde(rename = "organization_member_update_role")]
    Organization_member_update_role,
    #[serde(rename = "organization_member_update_mfa_method")]
    Organization_member_update_mfa_method,
    #[serde(rename = "user_login")]
    User_login,
    #[serde(rename = "user_login_failed")]
    User_login_failed,
    #[serde(rename = "user_logout")]
    User_logout,
    #[serde(rename = "key_create")]
    Key_create,
    #[serde(rename = "key_delete")]
    Key_delete,
    #[serde(rename = "openapi_key_update")]
    Openapi_key_update,
    #[serde(rename = "service_create")]
    Service_create,
    #[serde(rename = "service_start")]
    Service_start,
    #[serde(rename = "service_stop")]
    Service_stop,
    #[serde(rename = "service_awaken")]
    Service_awaken,
    #[serde(rename = "service_idle")]
    Service_idle,
    #[serde(rename = "service_running")]
    Service_running,
    #[serde(rename = "service_partially_running")]
    Service_partially_running,
    #[serde(rename = "service_delete")]
    Service_delete,
    #[serde(rename = "service_update_name")]
    Service_update_name,
    #[serde(rename = "service_update_ip_access_list")]
    Service_update_ip_access_list,
    #[serde(rename = "service_update_autoscaling_memory")]
    Service_update_autoscaling_memory,
    #[serde(rename = "service_update_autoscaling_idling")]
    Service_update_autoscaling_idling,
    #[serde(rename = "service_update_password")]
    Service_update_password,
    #[serde(rename = "service_update_autoscaling_replicas")]
    Service_update_autoscaling_replicas,
    #[serde(rename = "service_update_max_allowable_replicas")]
    Service_update_max_allowable_replicas,
    #[serde(rename = "service_update_backup_configuration")]
    Service_update_backup_configuration,
    #[serde(rename = "service_restore_backup")]
    Service_restore_backup,
    #[serde(rename = "service_update_release_channel")]
    Service_update_release_channel,
    #[serde(rename = "service_update_gpt_usage_consent")]
    Service_update_gpt_usage_consent,
    #[serde(rename = "service_update_private_endpoints")]
    Service_update_private_endpoints,
    #[serde(rename = "service_import_to_organization")]
    Service_import_to_organization,
    #[serde(rename = "service_export_from_organization")]
    Service_export_from_organization,
    #[serde(rename = "service_maintenance_start")]
    Service_maintenance_start,
    #[serde(rename = "service_maintenance_end")]
    Service_maintenance_end,
    #[serde(rename = "service_update_core_dump")]
    Service_update_core_dump,
    #[serde(rename = "backup_delete")]
    Backup_delete,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create_organization => write!(f, "create_organization"),
            Self::Organization_update_name => write!(f, "organization_update_name"),
            Self::Transfer_service_in => write!(f, "transfer_service_in"),
            Self::Transfer_service_out => write!(f, "transfer_service_out"),
            Self::Save_payment_method => write!(f, "save_payment_method"),
            Self::Marketplace_subscription => write!(f, "marketplace_subscription"),
            Self::Migrate_marketplace_billing_details_in => write!(f, "migrate_marketplace_billing_details_in"),
            Self::Migrate_marketplace_billing_details_out => write!(f, "migrate_marketplace_billing_details_out"),
            Self::Organization_update_tier => write!(f, "organization_update_tier"),
            Self::Organization_invite_create => write!(f, "organization_invite_create"),
            Self::Organization_invite_delete => write!(f, "organization_invite_delete"),
            Self::Organization_member_join => write!(f, "organization_member_join"),
            Self::Organization_member_add => write!(f, "organization_member_add"),
            Self::Organization_member_leave => write!(f, "organization_member_leave"),
            Self::Organization_member_delete => write!(f, "organization_member_delete"),
            Self::Organization_member_update_role => write!(f, "organization_member_update_role"),
            Self::Organization_member_update_mfa_method => write!(f, "organization_member_update_mfa_method"),
            Self::User_login => write!(f, "user_login"),
            Self::User_login_failed => write!(f, "user_login_failed"),
            Self::User_logout => write!(f, "user_logout"),
            Self::Key_create => write!(f, "key_create"),
            Self::Key_delete => write!(f, "key_delete"),
            Self::Openapi_key_update => write!(f, "openapi_key_update"),
            Self::Service_create => write!(f, "service_create"),
            Self::Service_start => write!(f, "service_start"),
            Self::Service_stop => write!(f, "service_stop"),
            Self::Service_awaken => write!(f, "service_awaken"),
            Self::Service_idle => write!(f, "service_idle"),
            Self::Service_running => write!(f, "service_running"),
            Self::Service_partially_running => write!(f, "service_partially_running"),
            Self::Service_delete => write!(f, "service_delete"),
            Self::Service_update_name => write!(f, "service_update_name"),
            Self::Service_update_ip_access_list => write!(f, "service_update_ip_access_list"),
            Self::Service_update_autoscaling_memory => write!(f, "service_update_autoscaling_memory"),
            Self::Service_update_autoscaling_idling => write!(f, "service_update_autoscaling_idling"),
            Self::Service_update_password => write!(f, "service_update_password"),
            Self::Service_update_autoscaling_replicas => write!(f, "service_update_autoscaling_replicas"),
            Self::Service_update_max_allowable_replicas => write!(f, "service_update_max_allowable_replicas"),
            Self::Service_update_backup_configuration => write!(f, "service_update_backup_configuration"),
            Self::Service_restore_backup => write!(f, "service_restore_backup"),
            Self::Service_update_release_channel => write!(f, "service_update_release_channel"),
            Self::Service_update_gpt_usage_consent => write!(f, "service_update_gpt_usage_consent"),
            Self::Service_update_private_endpoints => write!(f, "service_update_private_endpoints"),
            Self::Service_import_to_organization => write!(f, "service_import_to_organization"),
            Self::Service_export_from_organization => write!(f, "service_export_from_organization"),
            Self::Service_maintenance_start => write!(f, "service_maintenance_start"),
            Self::Service_maintenance_end => write!(f, "service_maintenance_end"),
            Self::Service_update_core_dump => write!(f, "service_update_core_dump"),
            Self::Backup_delete => write!(f, "backup_delete"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKey.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKeyPatchRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPatchRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyPatchRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKeyPostRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPostRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyPostRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AssignedRole.roleType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AssignedRoleRoletype {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AssignedRoleRoletype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketBucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPropertiesBucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketBucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPropertiesBucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Backup.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupStatus {
    #[serde(rename = "done")]
    #[default]
    Done,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "in_progress")]
    In_progress,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done => write!(f, "done"),
            Self::Error => write!(f, "error"),
            Self::In_progress => write!(f, "in_progress"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Backup.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupType {
    #[serde(rename = "full")]
    #[default]
    Full,
    #[serde(rename = "incremental")]
    Incremental,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Incremental => write!(f, "incremental"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigRegionid {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigRegionid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigState {
    #[serde(rename = "infra-ready")]
    #[default]
    Infra_ready,
    #[serde(rename = "infra-provisioning")]
    Infra_provisioning,
    #[serde(rename = "infra-terminated")]
    Infra_terminated,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infra_ready => write!(f, "infra-ready"),
            Self::Infra_provisioning => write!(f, "infra-provisioning"),
            Self::Infra_terminated => write!(f, "infra-terminated"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructurePostRequest.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructurePostRequestRegionid {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocInfrastructurePostRequestRegionid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipe.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeState {
    #[default]
    Unknown,
    Provisioning,
    Running,
    Stopping,
    Stopped,
    Failed,
    Completed,
    InternalError,
    Setup,
    Snapshot,
    Paused,
    Pausing,
    Modifying,
    Resync,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ClickPipeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Running => write!(f, "Running"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Failed => write!(f, "Failed"),
            Self::Completed => write!(f, "Completed"),
            Self::InternalError => write!(f, "InternalError"),
            Self::Setup => write!(f, "Setup"),
            Self::Snapshot => write!(f, "Snapshot"),
            Self::Paused => write!(f, "Paused"),
            Self::Pausing => write!(f, "Pausing"),
            Self::Modifying => write!(f, "Modifying"),
            Self::Resync => write!(f, "Resync"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeBigQueryPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeSettingsReplicationmode {
    #[serde(rename = "snapshot")]
    #[default]
    Snapshot,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot => write!(f, "snapshot"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeBigQueryPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeDestinationTableEngine.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeDestinationTableEngineType {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    SummingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeDestinationTableEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::SummingMergeTree => write!(f, "SummingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaOffset.strategy`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaOffsetStrategy {
    #[serde(rename = "from_beginning")]
    #[default]
    From_beginning,
    #[serde(rename = "from_latest")]
    From_latest,
    #[serde(rename = "from_timestamp")]
    From_timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaOffsetStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::From_beginning => write!(f, "from_beginning"),
            Self::From_latest => write!(f, "from_latest"),
            Self::From_timestamp => write!(f, "from_timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kafka => write!(f, "kafka"),
            Self::Redpanda => write!(f, "redpanda"),
            Self::Msk => write!(f, "msk"),
            Self::Gcmk => write!(f, "gcmk"),
            Self::Confluent => write!(f, "confluent"),
            Self::Warpstream => write!(f, "warpstream"),
            Self::Azureeventhub => write!(f, "azureeventhub"),
            Self::Dokafka => write!(f, "dokafka"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceIteratortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TRIM_HORIZON => write!(f, "TRIM_HORIZON"),
            Self::LATEST => write!(f, "LATEST"),
            Self::AT_TIMESTAMP => write!(f, "AT_TIMESTAMP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateKafkaSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutatePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutatePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutatePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutatePostgresSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
            Self::Neon => write!(f, "neon"),
            Self::Alloydb => write!(f, "alloydb"),
            Self::Planetscale => write!(f, "planetscale"),
            Self::Rdspostgres => write!(f, "rdspostgres"),
            Self::Aurorapostgres => write!(f, "aurorapostgres"),
            Self::Cloudsqlpostgres => write!(f, "cloudsqlpostgres"),
            Self::Azurepostgres => write!(f, "azurepostgres"),
            Self::Crunchybridge => write!(f, "crunchybridge"),
            Self::Tigerdata => write!(f, "tigerdata"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMechanism`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmechanism {
    #[default]
    GTID,
    FILE_POS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeSettingsReplicationmechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GTID => write!(f, "GTID"),
            Self::FILE_POS => write!(f, "FILE_POS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Gz => write!(f, "gz"),
            Self::Brotli => write!(f, "brotli"),
            Self::Br => write!(f, "br"),
            Self::Xz => write!(f, "xz"),
            Self::LZMA => write!(f, "LZMA"),
            Self::Zstd => write!(f, "zstd"),
            Self::Auto => write!(f, "auto"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::JSONAsObject => write!(f, "JSONAsObject"),
            Self::CSV => write!(f, "CSV"),
            Self::CSVWithNames => write!(f, "CSVWithNames"),
            Self::TabSeparated => write!(f, "TabSeparated"),
            Self::TabSeparatedWithNames => write!(f, "TabSeparatedWithNames"),
            Self::Parquet => write!(f, "Parquet"),
            Self::Avro => write!(f, "Avro"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Dospaces => write!(f, "dospaces"),
            Self::Azureblobstorage => write!(f, "azureblobstorage"),
            Self::Cloudflarer2 => write!(f, "cloudflarer2"),
            Self::Ovhobjectstorage => write!(f, "ovhobjectstorage"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMongoDBPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMySQLPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchPostgresPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kafka => write!(f, "kafka"),
            Self::Redpanda => write!(f, "redpanda"),
            Self::Msk => write!(f, "msk"),
            Self::Gcmk => write!(f, "gcmk"),
            Self::Confluent => write!(f, "confluent"),
            Self::Warpstream => write!(f, "warpstream"),
            Self::Azureeventhub => write!(f, "azureeventhub"),
            Self::Dokafka => write!(f, "dokafka"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceIteratortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TRIM_HORIZON => write!(f, "TRIM_HORIZON"),
            Self::LATEST => write!(f, "LATEST"),
            Self::AT_TIMESTAMP => write!(f, "AT_TIMESTAMP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Gz => write!(f, "gz"),
            Self::Brotli => write!(f, "brotli"),
            Self::Br => write!(f, "br"),
            Self::Xz => write!(f, "xz"),
            Self::LZMA => write!(f, "LZMA"),
            Self::Zstd => write!(f, "zstd"),
            Self::Auto => write!(f, "auto"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::JSONAsObject => write!(f, "JSONAsObject"),
            Self::CSV => write!(f, "CSV"),
            Self::CSVWithNames => write!(f, "CSVWithNames"),
            Self::TabSeparated => write!(f, "TabSeparated"),
            Self::TabSeparatedWithNames => write!(f, "TabSeparatedWithNames"),
            Self::Parquet => write!(f, "Parquet"),
            Self::Avro => write!(f, "Avro"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Dospaces => write!(f, "dospaces"),
            Self::Azureblobstorage => write!(f, "azureblobstorage"),
            Self::Cloudflarer2 => write!(f, "cloudflarer2"),
            Self::Ovhobjectstorage => write!(f, "ovhobjectstorage"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
            Self::Neon => write!(f, "neon"),
            Self::Alloydb => write!(f, "alloydb"),
            Self::Planetscale => write!(f, "planetscale"),
            Self::Rdspostgres => write!(f, "rdspostgres"),
            Self::Aurorapostgres => write!(f, "aurorapostgres"),
            Self::Cloudsqlpostgres => write!(f, "cloudsqlpostgres"),
            Self::Azurepostgres => write!(f, "azurepostgres"),
            Self::Crunchybridge => write!(f, "crunchybridge"),
            Self::Tigerdata => write!(f, "tigerdata"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "resync")]
    Resync,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeStatePatchRequestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Resync => write!(f, "resync"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelEmail.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelEmailType {
    #[serde(rename = "webhook")]
    #[default]
    Webhook,
    #[serde(rename = "email")]
    Email,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelEmailType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Webhook => write!(f, "webhook"),
            Self::Email => write!(f, "email"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelWebhook.severity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookSeverity {
    #[serde(rename = "critical")]
    #[default]
    Critical,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelWebhookSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelWebhook.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookType {
    #[serde(rename = "webhook")]
    #[default]
    Webhook,
    #[serde(rename = "email")]
    Email,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelWebhookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Webhook => write!(f, "webhook"),
            Self::Email => write!(f, "email"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseState {
    #[default]
    ALERT,
    OK,
    INSUFFICIENT_DATA,
    DISABLED,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALERT => write!(f, "ALERT"),
            Self::OK => write!(f, "OK"),
            Self::INSUFFICIENT_DATA => write!(f, "INSUFFICIENT_DATA"),
            Self::DISABLED => write!(f, "DISABLED"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarBuilderChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateDashboardRequestSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackDashboardResponse.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackDashboardResponseSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackDashboardResponseSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterSourcemetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterType {
    #[default]
    QUERY_EXPRESSION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_EXPRESSION => write!(f, "QUERY_EXPRESSION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputSourcemetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputType {
    #[default]
    QUERY_EXPRESSION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_EXPRESSION => write!(f, "QUERY_EXPRESSION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackGenericWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackGenericWebhookService {
    #[serde(rename = "generic")]
    #[default]
    Generic,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackGenericWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic => write!(f, "generic"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackIncidentIOWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackIncidentIOWebhookService {
    #[serde(rename = "incidentio")]
    #[default]
    Incidentio,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackIncidentIOWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incidentio => write!(f, "incidentio"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineBuilderChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLogSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLogSourceKind {
    #[serde(rename = "log")]
    #[default]
    Log,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLogSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log => write!(f, "log"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMarkdownChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartConfigDisplaytype {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMarkdownChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMarkdownChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartSeriesType {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMarkdownChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMaterializedView.minGranularity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMaterializedViewMingranularity {
    #[serde(rename = "1s")]
    #[default]
    _1s,
    #[serde(rename = "15s")]
    _15s,
    #[serde(rename = "30s")]
    _30s,
    #[serde(rename = "1m")]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "2h")]
    _2h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    #[serde(rename = "2d")]
    _2d,
    #[serde(rename = "7d")]
    _7d,
    #[serde(rename = "30d")]
    _30d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMaterializedViewMingranularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1s => write!(f, "1s"),
            Self::_15s => write!(f, "15s"),
            Self::_30s => write!(f, "30s"),
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_2h => write!(f, "2h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::_2d => write!(f, "2d"),
            Self::_7d => write!(f, "7d"),
            Self::_30d => write!(f, "30d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMetricSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMetricSourceKind {
    #[serde(rename = "metric")]
    #[default]
    Metric,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMetricSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Metric => write!(f, "metric"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberBuilderChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesType {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberFormat.output`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberFormatOutput {
    #[serde(rename = "currency")]
    #[default]
    Currency,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "byte")]
    Byte,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "number")]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberFormatOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Currency => write!(f, "currency"),
            Self::Percent => write!(f, "percent"),
            Self::Byte => write!(f, "byte"),
            Self::Time => write!(f, "time"),
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPagerDutyAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPagerDutyAPIWebhookService {
    #[serde(rename = "pagerduty_api")]
    #[default]
    Pagerduty_api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPagerDutyAPIWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pagerduty_api => write!(f, "pagerduty_api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieBuilderChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pie => write!(f, "pie"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pie => write!(f, "pie"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedFilterValue.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedFilterValueType {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedFilterValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigDisplaytype {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesType {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.level`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemLevel {
    #[serde(rename = "0.5")]
    #[default]
    _0_5,
    #[serde(rename = "0.9")]
    _0_9,
    #[serde(rename = "0.95")]
    _0_95,
    #[serde(rename = "0.99")]
    _0_99,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_0_5 => write!(f, "0.5"),
            Self::_0_9 => write!(f, "0.9"),
            Self::_0_95 => write!(f, "0.95"),
            Self::_0_99 => write!(f, "0.99"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.metricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemMetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemMetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.periodAggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemPeriodaggfn {
    #[serde(rename = "delta")]
    #[default]
    Delta,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemPeriodaggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delta => write!(f, "delta"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSessionSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSessionSourceKind {
    #[serde(rename = "session")]
    #[default]
    Session,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSessionSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session => write!(f, "session"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSlackAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackAPIWebhookService {
    #[serde(rename = "slack_api")]
    #[default]
    Slack_api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSlackAPIWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack_api => write!(f, "slack_api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSlackWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackWebhookService {
    #[serde(rename = "slack")]
    #[default]
    Slack,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSlackWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableBuilderChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.sortOrder`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesSortorder {
    #[serde(rename = "desc")]
    #[default]
    Desc,
    #[serde(rename = "asc")]
    Asc,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesSortorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Desc => write!(f, "desc"),
            Self::Asc => write!(f, "asc"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesType {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    #[serde(rename = "line")]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesType {
    #[serde(rename = "time")]
    #[default]
    Time,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Time => write!(f, "time"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTraceSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTraceSourceKind {
    #[serde(rename = "trace")]
    #[default]
    Trace,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTraceSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "trace"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateDashboardRequestSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `CreateReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `CreateReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketBucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPostRequestV1Bucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPropertiesBucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InstancePrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InstancePrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InstancePrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InstancePrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InstancePrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InstancePrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Invitation.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InvitationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InvitationPostRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationPostRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InvitationPostRequestRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Member.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `MemberPatchRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberPatchRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for MemberPatchRequestRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPatchPrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPatchPrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPatchPrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPatchPrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPatchPrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPatchPrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `PostgresServiceSetState.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PostgresServiceSetStateCommand {
    #[serde(rename = "restart")]
    #[default]
    Restart,
    #[serde(rename = "promote")]
    Promote,
    #[serde(rename = "switchover")]
    Switchover,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PostgresServiceSetStateCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restart => write!(f, "restart"),
            Self::Promote => write!(f, "promote"),
            Self::Switchover => write!(f, "switchover"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicy.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyAllowdeny {
    #[default]
    ALLOW,
    DENY,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyAllowdeny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALLOW => write!(f, "ALLOW"),
            Self::DENY => write!(f, "DENY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicyCreateRequest.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyCreateRequestAllowdeny {
    #[default]
    ALLOW,
    DENY,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyCreateRequestAllowdeny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALLOW => write!(f, "ALLOW"),
            Self::DENY => write!(f, "DENY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicyTags.roleV2`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyTagsRolev2 {
    #[serde(rename = "sql-console-readonly")]
    #[default]
    Sql_console_readonly,
    #[serde(rename = "sql-console-admin")]
    Sql_console_admin,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyTagsRolev2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql_console_readonly => write!(f, "sql-console-readonly"),
            Self::Sql_console_admin => write!(f, "sql-console-admin"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACRole.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACRoleType {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACRoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointStatus {
    #[default]
    Unknown,
    Provisioning,
    Deleting,
    Ready,
    Failed,
    PendingAcceptance,
    Rejected,
    Expired,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ReversePrivateEndpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Deleting => write!(f, "Deleting"),
            Self::Ready => write!(f, "Ready"),
            Self::Failed => write!(f, "Failed"),
            Self::PendingAcceptance => write!(f, "PendingAcceptance"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ScimPatchOperation.op`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ScimPatchOperationOp {
    #[serde(rename = "add")]
    #[default]
    Add,
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "remove")]
    Remove,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ScimPatchOperationOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Replace => write!(f, "replace"),
            Self::Remove => write!(f, "remove"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceState {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "terminating")]
    Terminating,
    #[serde(rename = "softdeleting")]
    Softdeleting,
    #[serde(rename = "awaking")]
    Awaking,
    #[serde(rename = "partially_running")]
    Partially_running,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "terminated")]
    Terminated,
    #[serde(rename = "softdeleted")]
    Softdeleted,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "idle")]
    Idle,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminating => write!(f, "terminating"),
            Self::Softdeleting => write!(f, "softdeleting"),
            Self::Awaking => write!(f, "awaking"),
            Self::Partially_running => write!(f, "partially_running"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
            Self::Softdeleted => write!(f, "softdeleted"),
            Self::Degraded => write!(f, "degraded"),
            Self::Failed => write!(f, "failed"),
            Self::Idle => write!(f, "idle"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => write!(f, "dedicated_standard_n2d_standard_4"),
            Self::Dedicated_standard_n2d_standard_8 => write!(f, "dedicated_standard_n2d_standard_8"),
            Self::Dedicated_standard_n2d_standard_32 => write!(f, "dedicated_standard_n2d_standard_32"),
            Self::Dedicated_standard_n2d_standard_128 => write!(f, "dedicated_standard_n2d_standard_128"),
            Self::Dedicated_standard_n2d_standard_32_16SSD => write!(f, "dedicated_standard_n2d_standard_32_16SSD"),
            Self::Dedicated_standard_n2d_standard_64_24SSD => write!(f, "dedicated_standard_n2d_standard_64_24SSD"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceEndpoint.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointProtocol {
    #[serde(rename = "https")]
    #[default]
    Https,
    #[serde(rename = "nativesecure")]
    Nativesecure,
    #[serde(rename = "mysql")]
    Mysql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceEndpointProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Https => write!(f, "https"),
            Self::Nativesecure => write!(f, "nativesecure"),
            Self::Mysql => write!(f, "mysql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceEndpointChange.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointChangeProtocol {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceEndpointChangeProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePatchRequest.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePatchRequestReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePatchRequestReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePostRequest.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePostRequest.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePostRequest.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePostRequest.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePostRequest.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePostRequest.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => write!(f, "dedicated_standard_n2d_standard_4"),
            Self::Dedicated_standard_n2d_standard_8 => write!(f, "dedicated_standard_n2d_standard_8"),
            Self::Dedicated_standard_n2d_standard_32 => write!(f, "dedicated_standard_n2d_standard_32"),
            Self::Dedicated_standard_n2d_standard_128 => write!(f, "dedicated_standard_n2d_standard_128"),
            Self::Dedicated_standard_n2d_standard_32_16SSD => write!(f, "dedicated_standard_n2d_standard_32_16SSD"),
            Self::Dedicated_standard_n2d_standard_64_24SSD => write!(f, "dedicated_standard_n2d_standard_64_24SSD"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseState {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "terminating")]
    Terminating,
    #[serde(rename = "softdeleting")]
    Softdeleting,
    #[serde(rename = "awaking")]
    Awaking,
    #[serde(rename = "partially_running")]
    Partially_running,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "terminated")]
    Terminated,
    #[serde(rename = "softdeleted")]
    Softdeleted,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "idle")]
    Idle,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminating => write!(f, "terminating"),
            Self::Softdeleting => write!(f, "softdeleting"),
            Self::Awaking => write!(f, "awaking"),
            Self::Partially_running => write!(f, "partially_running"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
            Self::Softdeleted => write!(f, "softdeleted"),
            Self::Degraded => write!(f, "degraded"),
            Self::Failed => write!(f, "failed"),
            Self::Idle => write!(f, "idle"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => write!(f, "dedicated_standard_n2d_standard_4"),
            Self::Dedicated_standard_n2d_standard_8 => write!(f, "dedicated_standard_n2d_standard_8"),
            Self::Dedicated_standard_n2d_standard_32 => write!(f, "dedicated_standard_n2d_standard_32"),
            Self::Dedicated_standard_n2d_standard_128 => write!(f, "dedicated_standard_n2d_standard_128"),
            Self::Dedicated_standard_n2d_standard_32_16SSD => write!(f, "dedicated_standard_n2d_standard_32_16SSD"),
            Self::Dedicated_standard_n2d_standard_64_24SSD => write!(f, "dedicated_standard_n2d_standard_64_24SSD"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "awake")]
    Awake,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceStatePatchRequestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Awake => write!(f, "awake"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `UsageCostRecord.entityType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UsageCostRecordEntitytype {
    #[serde(rename = "datawarehouse")]
    #[default]
    Datawarehouse,
    #[serde(rename = "service")]
    Service,
    #[serde(rename = "clickpipe")]
    Clickpipe,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UsageCostRecordEntitytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Datawarehouse => write!(f, "datawarehouse"),
            Self::Service => write!(f, "service"),
            Self::Clickpipe => write!(f, "clickpipe"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `pgConfig.default_transaction_isolation`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigDefaultTransactionIsolation {
    #[serde(rename = "read committed")]
    #[default]
    Read_committed,
    #[serde(rename = "repeatable read")]
    Repeatable_read,
    #[serde(rename = "serializable")]
    Serializable,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgConfigDefaultTransactionIsolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read_committed => write!(f, "read committed"),
            Self::Repeatable_read => write!(f, "repeatable read"),
            Self::Serializable => write!(f, "serializable"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `pgConfig.wal_compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigWalCompression {
    #[serde(rename = "off")]
    #[default]
    Off,
    #[serde(rename = "on")]
    On,
    #[serde(rename = "lz4")]
    Lz4,
    #[serde(rename = "zstd")]
    Zstd,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgConfigWalCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::On => write!(f, "on"),
            Self::Lz4 => write!(f, "lz4"),
            Self::Zstd => write!(f, "zstd"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucket` - one of multiple variants.
///
/// Uses `bucketProvider` as a discriminator: `"AWS"`, `"GCP"`, or `"AZURE"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucket {
    AwsBackupBucket(AwsBackupBucket),
    GcpBackupBucket(GcpBackupBucket),
    AzureBackupBucket(AzureBackupBucket),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl<'de> Deserialize<'de> for BackupBucket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("bucketProvider").and_then(|v| v.as_str()) {
            Some("AWS") => serde_json::from_value(value)
                .map(BackupBucket::AwsBackupBucket)
                .map_err(serde::de::Error::custom),
            Some("GCP") => serde_json::from_value(value)
                .map(BackupBucket::GcpBackupBucket)
                .map_err(serde::de::Error::custom),
            Some("AZURE") => serde_json::from_value(value)
                .map(BackupBucket::AzureBackupBucket)
                .map_err(serde::de::Error::custom),
            _ => Ok(BackupBucket::Unknown(value.to_string())),
        }
    }
}

impl std::fmt::Display for BackupBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucket(_) => write!(f, "AwsBackupBucket"),
            Self::GcpBackupBucket(_) => write!(f, "GcpBackupBucket"),
            Self::AzureBackupBucket(_) => write!(f, "AzureBackupBucket"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketPatchRequest` - one of multiple variants.
///
/// Uses `bucketProvider` as a discriminator: `"AWS"`, `"GCP"`, or `"AZURE"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketPatchRequest {
    AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1),
    GcpBackupBucketPatchRequestV1(GcpBackupBucketPatchRequestV1),
    AzureBackupBucketPatchRequestV1(AzureBackupBucketPatchRequestV1),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl<'de> Deserialize<'de> for BackupBucketPatchRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("bucketProvider").and_then(|v| v.as_str()) {
            Some("AWS") => serde_json::from_value(value)
                .map(BackupBucketPatchRequest::AwsBackupBucketPatchRequestV1)
                .map_err(serde::de::Error::custom),
            Some("GCP") => serde_json::from_value(value)
                .map(BackupBucketPatchRequest::GcpBackupBucketPatchRequestV1)
                .map_err(serde::de::Error::custom),
            Some("AZURE") => serde_json::from_value(value)
                .map(BackupBucketPatchRequest::AzureBackupBucketPatchRequestV1)
                .map_err(serde::de::Error::custom),
            _ => Ok(BackupBucketPatchRequest::Unknown(value.to_string())),
        }
    }
}

impl std::fmt::Display for BackupBucketPatchRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketPatchRequestV1(_) => write!(f, "AwsBackupBucketPatchRequestV1"),
            Self::GcpBackupBucketPatchRequestV1(_) => write!(f, "GcpBackupBucketPatchRequestV1"),
            Self::AzureBackupBucketPatchRequestV1(_) => write!(f, "AzureBackupBucketPatchRequestV1"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketPostRequest` - one of multiple variants.
///
/// Uses `bucketProvider` as a discriminator: `"AWS"`, `"GCP"`, or `"AZURE"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketPostRequest {
    AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1),
    GcpBackupBucketPostRequestV1(GcpBackupBucketPostRequestV1),
    AzureBackupBucketPostRequestV1(AzureBackupBucketPostRequestV1),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl<'de> Deserialize<'de> for BackupBucketPostRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("bucketProvider").and_then(|v| v.as_str()) {
            Some("AWS") => serde_json::from_value(value)
                .map(BackupBucketPostRequest::AwsBackupBucketPostRequestV1)
                .map_err(serde::de::Error::custom),
            Some("GCP") => serde_json::from_value(value)
                .map(BackupBucketPostRequest::GcpBackupBucketPostRequestV1)
                .map_err(serde::de::Error::custom),
            Some("AZURE") => serde_json::from_value(value)
                .map(BackupBucketPostRequest::AzureBackupBucketPostRequestV1)
                .map_err(serde::de::Error::custom),
            _ => Ok(BackupBucketPostRequest::Unknown(value.to_string())),
        }
    }
}

impl std::fmt::Display for BackupBucketPostRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketPostRequestV1(_) => write!(f, "AwsBackupBucketPostRequestV1"),
            Self::GcpBackupBucketPostRequestV1(_) => write!(f, "GcpBackupBucketPostRequestV1"),
            Self::AzureBackupBucketPostRequestV1(_) => write!(f, "AzureBackupBucketPostRequestV1"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketProperties` - one of multiple variants.
///
/// Uses `bucketProvider` as a discriminator: `"AWS"`, `"GCP"`, or `"AZURE"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketProperties {
    AwsBackupBucketProperties(AwsBackupBucketProperties),
    GcpBackupBucketProperties(GcpBackupBucketProperties),
    AzureBackupBucketProperties(AzureBackupBucketProperties),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl<'de> Deserialize<'de> for BackupBucketProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.get("bucketProvider").and_then(|v| v.as_str()) {
            Some("AWS") => serde_json::from_value(value)
                .map(BackupBucketProperties::AwsBackupBucketProperties)
                .map_err(serde::de::Error::custom),
            Some("GCP") => serde_json::from_value(value)
                .map(BackupBucketProperties::GcpBackupBucketProperties)
                .map_err(serde::de::Error::custom),
            Some("AZURE") => serde_json::from_value(value)
                .map(BackupBucketProperties::AzureBackupBucketProperties)
                .map_err(serde::de::Error::custom),
            _ => Ok(BackupBucketProperties::Unknown(value.to_string())),
        }
    }
}

impl std::fmt::Display for BackupBucketProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketProperties(_) => write!(f, "AwsBackupBucketProperties"),
            Self::GcpBackupBucketProperties(_) => write!(f, "GcpBackupBucketProperties"),
            Self::AzureBackupBucketProperties(_) => write!(f, "AzureBackupBucketProperties"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertChannel` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackAlertChannel {
    ClickStackAlertChannelEmail(ClickStackAlertChannelEmail),
    ClickStackAlertChannelWebhook(ClickStackAlertChannelWebhook),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertChannelEmail(_) => write!(f, "ClickStackAlertChannelEmail"),
            Self::ClickStackAlertChannelWebhook(_) => write!(f, "ClickStackAlertChannelWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackBarChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackBarChartConfig {
    ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig),
    ClickStackBarRawSqlChartConfig(ClickStackBarRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackBarBuilderChartConfig(_) => write!(f, "ClickStackBarBuilderChartConfig"),
            Self::ClickStackBarRawSqlChartConfig(_) => write!(f, "ClickStackBarRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackDashboardChartSeries` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackDashboardChartSeries {
    ClickStackTimeChartSeries(ClickStackTimeChartSeries),
    ClickStackTableChartSeries(ClickStackTableChartSeries),
    ClickStackNumberChartSeries(ClickStackNumberChartSeries),
    ClickStackSearchChartSeries(ClickStackSearchChartSeries),
    ClickStackMarkdownChartSeries(ClickStackMarkdownChartSeries),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackDashboardChartSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTimeChartSeries(_) => write!(f, "ClickStackTimeChartSeries"),
            Self::ClickStackTableChartSeries(_) => write!(f, "ClickStackTableChartSeries"),
            Self::ClickStackNumberChartSeries(_) => write!(f, "ClickStackNumberChartSeries"),
            Self::ClickStackSearchChartSeries(_) => write!(f, "ClickStackSearchChartSeries"),
            Self::ClickStackMarkdownChartSeries(_) => write!(f, "ClickStackMarkdownChartSeries"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackLineChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackLineChartConfig {
    ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig),
    ClickStackLineRawSqlChartConfig(ClickStackLineRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineBuilderChartConfig(_) => write!(f, "ClickStackLineBuilderChartConfig"),
            Self::ClickStackLineRawSqlChartConfig(_) => write!(f, "ClickStackLineRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackNumberChartConfig {
    ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig),
    ClickStackNumberRawSqlChartConfig(ClickStackNumberRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumberBuilderChartConfig(_) => write!(f, "ClickStackNumberBuilderChartConfig"),
            Self::ClickStackNumberRawSqlChartConfig(_) => write!(f, "ClickStackNumberRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackPieChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackPieChartConfig {
    ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig),
    ClickStackPieRawSqlChartConfig(ClickStackPieRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackPieBuilderChartConfig(_) => write!(f, "ClickStackPieBuilderChartConfig"),
            Self::ClickStackPieRawSqlChartConfig(_) => write!(f, "ClickStackPieRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackSource` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackSource {
    ClickStackLogSource(ClickStackLogSource),
    ClickStackTraceSource(ClickStackTraceSource),
    ClickStackMetricSource(ClickStackMetricSource),
    ClickStackSessionSource(ClickStackSessionSource),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLogSource(_) => write!(f, "ClickStackLogSource"),
            Self::ClickStackTraceSource(_) => write!(f, "ClickStackTraceSource"),
            Self::ClickStackMetricSource(_) => write!(f, "ClickStackMetricSource"),
            Self::ClickStackSessionSource(_) => write!(f, "ClickStackSessionSource"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTableChartConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackTableChartConfig {
    ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig),
    ClickStackTableRawSqlChartConfig(ClickStackTableRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTableBuilderChartConfig(_) => write!(f, "ClickStackTableBuilderChartConfig"),
            Self::ClickStackTableRawSqlChartConfig(_) => write!(f, "ClickStackTableRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTileConfig` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackTileConfig {
    ClickStackLineChartConfig(ClickStackLineChartConfig),
    ClickStackBarChartConfig(ClickStackBarChartConfig),
    ClickStackTableChartConfig(ClickStackTableChartConfig),
    ClickStackNumberChartConfig(ClickStackNumberChartConfig),
    ClickStackPieChartConfig(ClickStackPieChartConfig),
    ClickStackSearchChartConfig(ClickStackSearchChartConfig),
    ClickStackMarkdownChartConfig(ClickStackMarkdownChartConfig),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackTileConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineChartConfig(_) => write!(f, "ClickStackLineChartConfig"),
            Self::ClickStackBarChartConfig(_) => write!(f, "ClickStackBarChartConfig"),
            Self::ClickStackTableChartConfig(_) => write!(f, "ClickStackTableChartConfig"),
            Self::ClickStackNumberChartConfig(_) => write!(f, "ClickStackNumberChartConfig"),
            Self::ClickStackPieChartConfig(_) => write!(f, "ClickStackPieChartConfig"),
            Self::ClickStackSearchChartConfig(_) => write!(f, "ClickStackSearchChartConfig"),
            Self::ClickStackMarkdownChartConfig(_) => write!(f, "ClickStackMarkdownChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackWebhook` - one of multiple variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackWebhook {
    ClickStackSlackWebhook(ClickStackSlackWebhook),
    ClickStackIncidentIOWebhook(ClickStackIncidentIOWebhook),
    ClickStackGenericWebhook(ClickStackGenericWebhook),
    ClickStackSlackAPIWebhook(ClickStackSlackAPIWebhook),
    ClickStackPagerDutyAPIWebhook(ClickStackPagerDutyAPIWebhook),
    /// Catch-all for unknown or newly-added values.
    Unknown(String),
}

impl std::fmt::Display for ClickStackWebhook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackSlackWebhook(_) => write!(f, "ClickStackSlackWebhook"),
            Self::ClickStackIncidentIOWebhook(_) => write!(f, "ClickStackIncidentIOWebhook"),
            Self::ClickStackGenericWebhook(_) => write!(f, "ClickStackGenericWebhook"),
            Self::ClickStackSlackAPIWebhook(_) => write!(f, "ClickStackSlackAPIWebhook"),
            Self::ClickStackPagerDutyAPIWebhook(_) => write!(f, "ClickStackPagerDutyAPIWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Type alias for `pgCreatedAtProperty`.
pub type PgCreatedAtProperty = chrono::DateTime<chrono::Utc>;

/// Type alias for `pgIdProperty`.
pub type PgIdProperty = uuid::Uuid;

/// Type alias for `pgIsPrimaryProperty`.
pub type PgIsPrimaryProperty = bool;

/// Type alias for `pgNameProperty`.
pub type PgNameProperty = String;

/// Type alias for `pgPassword`.
pub type PgPassword = String;

/// Type alias for `pgPitrRestoreTargetProperty`.
pub type PgPitrRestoreTargetProperty = chrono::DateTime<chrono::Utc>;

/// Type alias for `pgRegion`.
pub type PgRegion = String;

/// Type alias for `pgStorageSize`.
pub type PgStorageSize = i64;

/// Type alias for `pgTags`.
pub type PgTags = Vec<ResourceTagsV1>;

/// `Activity` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "actorDetails", default)]
    pub actor_details: String,
    #[serde(rename = "actorId", default)]
    pub actor_id: String,
    #[serde(rename = "actorIpAddress", default)]
    pub actor_ip_address: String,
    #[serde(rename = "actorType", default)]
    pub actor_type: ActivityActortype,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub id: String,
    #[serde(rename = "keyUpdateType", default)]
    pub key_update_type: ActivityKeyupdatetype,
    #[serde(rename = "organizationId", default)]
    pub organization_id: String,
    #[serde(rename = "serviceId", default)]
    pub service_id: String,
    #[serde(rename = "targetKeyId", default)]
    pub target_key_id: String,
    #[serde(default)]
    pub r#type: ActivityType,
    #[serde(rename = "userAgent", default)]
    pub user_agent: String,
}

/// `ApiKey` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKey {
    #[serde(rename = "assignedRoles", default)]
    pub assigned_roles: Vec<AssignedRole>,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(rename = "ipAccessList", default)]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(rename = "keySuffix", default)]
    pub key_suffix: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub state: ApiKeyState,
    #[serde(rename = "usedAt", default)]
    pub used_at: chrono::DateTime<chrono::Utc>,
}

/// `ApiKeyHashData` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyHashData {
    #[serde(rename = "keyIdHash", default)]
    pub key_id_hash: String,
    #[serde(rename = "keyIdSuffix", default)]
    pub key_id_suffix: String,
    #[serde(rename = "keySecretHash", default)]
    pub key_secret_hash: String,
}

/// `ApiKeyPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none", default)]
    pub assigned_role_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ApiKeyPatchRequestState>,
}

/// `ApiKeyPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostRequest {
    #[serde(rename = "assignedRoleIds", default)]
    pub assigned_role_ids: Vec<uuid::Uuid>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none", default)]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "hashData", default)]
    pub hash_data: ApiKeyHashData,
    #[serde(rename = "ipAccessList", default)]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub state: ApiKeyPostRequestState,
}

/// `ApiKeyPostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostResponse {
    #[serde(default)]
    pub key: ApiKey,
    #[serde(rename = "keyId", default)]
    pub key_id: String,
    #[serde(rename = "keySecret", default)]
    pub key_secret: String,
}

/// `AssignedRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssignedRole {
    #[serde(rename = "roleId", default)]
    pub role_id: uuid::Uuid,
    #[serde(rename = "roleName", default)]
    pub role_name: String,
    #[serde(rename = "roleType", default)]
    pub role_type: AssignedRoleRoletype,
}

/// `AwsBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucket {
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AwsBackupBucketBucketprovider,
    #[serde(rename = "iamRoleArn", default)]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName", default)]
    pub iam_role_session_name: String,
    #[serde(default)]
    pub id: uuid::Uuid,
}

/// `AwsBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AwsBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "iamRoleArn", default)]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none", default)]
    pub iam_role_session_name: Option<String>,
}

/// `AwsBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPostRequestV1 {
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AwsBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "iamRoleArn", default)]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName", default)]
    pub iam_role_session_name: String,
}

/// `AwsBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketProperties {
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AwsBackupBucketPropertiesBucketprovider,
    #[serde(rename = "iamRoleArn", default)]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName", default)]
    pub iam_role_session_name: String,
}

/// `AzureBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucket {
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AzureBackupBucketBucketprovider,
    #[serde(rename = "containerName", default)]
    pub container_name: String,
    #[serde(default)]
    pub id: uuid::Uuid,
}

/// `AzureBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AzureBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "connectionString", default)]
    pub connection_string: String,
    #[serde(rename = "containerName", default)]
    pub container_name: String,
}

/// `AzureBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPostRequestV1 {
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AzureBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "connectionString", default)]
    pub connection_string: String,
    #[serde(rename = "containerName", default)]
    pub container_name: String,
}

/// `AzureBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketProperties {
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: AzureBackupBucketPropertiesBucketprovider,
    #[serde(rename = "containerName", default)]
    pub container_name: String,
}

/// `AzureEventHub` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureEventHub {
    #[serde(rename = "connectionString", default)]
    pub connection_string: String,
}

/// `Backup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Backup {
    #[serde(rename = "backupName", default)]
    pub backup_name: String,
    #[serde(default)]
    pub bucket: serde_json::Value,
    #[serde(rename = "durationInSeconds", default)]
    pub duration_in_seconds: f64,
    #[serde(rename = "finishedAt", default)]
    pub finished_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(rename = "serviceId", default)]
    pub service_id: String,
    #[serde(rename = "sizeInBytes", default)]
    pub size_in_bytes: f64,
    #[serde(rename = "startedAt", default)]
    pub started_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub status: BackupStatus,
    #[serde(default)]
    pub r#type: BackupType,
}

/// `BackupConfiguration` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfiguration {
    #[serde(rename = "backupPeriodInHours", default)]
    pub backup_period_in_hours: f64,
    #[serde(rename = "backupRetentionPeriodInHours", default)]
    pub backup_retention_period_in_hours: f64,
    #[serde(rename = "backupStartTime", default)]
    pub backup_start_time: String,
}

/// `BackupConfigurationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfigurationPatchRequest {
    #[serde(rename = "backupPeriodInHours", skip_serializing_if = "Option::is_none", default)]
    pub backup_period_in_hours: Option<f64>,
    #[serde(rename = "backupRetentionPeriodInHours", skip_serializing_if = "Option::is_none", default)]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none", default)]
    pub backup_start_time: Option<String>,
}

/// `BasePostgresService` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BasePostgresService {
    #[serde(rename = "haType", default)]
    pub ha_type: PgHaType,
    #[serde(default)]
    pub name: PgNameProperty,
    #[serde(rename = "postgresVersion", default)]
    pub postgres_version: PgVersion,
    #[serde(default)]
    pub provider: PgProvider,
    #[serde(default)]
    pub region: PgRegion,
    #[serde(default)]
    pub size: PgSize,
    #[serde(rename = "storageSize", default)]
    pub storage_size: PgStorageSize,
    #[serde(default)]
    pub tags: PgTags,
}

/// `ByocConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConfig {
    #[serde(rename = "accountName", default)]
    pub account_name: String,
    #[serde(rename = "cloudProvider", default)]
    pub cloud_provider: ByocConfigCloudprovider,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub id: String,
    #[serde(rename = "regionId", default)]
    pub region_id: ByocConfigRegionid,
    #[serde(default)]
    pub state: ByocConfigState,
}

/// `ByocInfrastructurePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePatchRequest {
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
}

/// `ByocInfrastructurePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePostRequest {
    #[serde(rename = "accountId", default)]
    pub account_id: String,
    #[serde(rename = "availabilityZoneSuffixes", default)]
    pub availability_zone_suffixes: Vec<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(rename = "regionId", default)]
    pub region_id: ByocInfrastructurePostRequestRegionid,
    #[serde(rename = "vpcCidrRange", default)]
    pub vpc_cidr_range: String,
}

/// `ClickPipe` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipe {
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub destination: ClickPipeDestination,
    #[serde(rename = "fieldMappings", default)]
    pub field_mappings: Vec<ClickPipeFieldMapping>,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub scaling: ClickPipeScaling,
    #[serde(rename = "serviceId", default)]
    pub service_id: uuid::Uuid,
    #[serde(default)]
    pub settings: ClickPipeSettings,
    #[serde(default)]
    pub source: ClickPipeSource,
    #[serde(default)]
    pub state: ClickPipeState,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettings {
    #[serde(rename = "allowNullableColumns", default)]
    pub allow_nullable_columns: bool,
    #[serde(rename = "initialLoadParallelism", default)]
    pub initial_load_parallelism: f64,
    #[serde(rename = "replicationMode", default)]
    pub replication_mode: ClickPipeBigQueryPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition", default)]
    pub snapshot_num_rows_per_partition: f64,
    #[serde(rename = "snapshotNumberOfParallelTables", default)]
    pub snapshot_number_of_parallel_tables: f64,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMapping {
    #[serde(rename = "excludedColumns", default)]
    pub excluded_columns: Vec<String>,
    #[serde(rename = "sortingKeys", default)]
    pub sorting_keys: Vec<String>,
    #[serde(rename = "sourceDatasetName", default)]
    pub source_dataset_name: String,
    #[serde(rename = "sourceTable", default)]
    pub source_table: String,
    #[serde(rename = "tableEngine", default)]
    pub table_engine: ClickPipeBigQueryPipeTableMappingTableengine,
    #[serde(rename = "targetTable", default)]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey", default)]
    pub use_custom_sorting_key: bool,
}

/// `ClickPipeBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQuerySource {
    #[serde(default)]
    pub settings: ClickPipeBigQueryPipeSettings,
    #[serde(rename = "snapshotStagingPath", default)]
    pub snapshot_staging_path: String,
    #[serde(rename = "tableMappings", default)]
    pub table_mappings: Vec<ClickPipeBigQueryPipeTableMapping>,
}

/// `ClickPipeDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestination {
    #[serde(default)]
    pub columns: Vec<ClickPipeDestinationColumn>,
    #[serde(default)]
    pub database: String,
    #[serde(rename = "managedTable", default)]
    pub managed_table: bool,
    #[serde(default)]
    pub table: String,
    #[serde(rename = "tableDefinition", default)]
    pub table_definition: ClickPipeDestinationTableDefinition,
}

/// `ClickPipeDestinationColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationColumn {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub r#type: String,
}

/// `ClickPipeDestinationTableDefinition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableDefinition {
    #[serde(default)]
    pub engine: ClickPipeDestinationTableEngine,
    #[serde(rename = "partitionBy", default)]
    pub partition_by: String,
    #[serde(rename = "primaryKey", default)]
    pub primary_key: String,
    #[serde(rename = "sortingKey", default)]
    pub sorting_key: Vec<String>,
}

/// `ClickPipeDestinationTableEngine` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableEngine {
    #[serde(rename = "columnIds", default)]
    pub column_ids: Vec<String>,
    #[serde(default)]
    pub r#type: ClickPipeDestinationTableEngineType,
    #[serde(rename = "versionColumnId", skip_serializing_if = "Option::is_none", default)]
    pub version_column_id: Option<String>,
}

/// `ClickPipeFieldMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeFieldMapping {
    #[serde(rename = "destinationField", default)]
    pub destination_field: String,
    #[serde(rename = "sourceField", default)]
    pub source_field: String,
}

/// `ClickPipeKafkaOffset` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaOffset {
    #[serde(default)]
    pub strategy: ClickPipeKafkaOffsetStrategy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<String>,
}

/// `ClickPipeKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistry {
    #[serde(default)]
    pub authentication: ClickPipeKafkaSchemaRegistryAuthentication,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(default)]
    pub url: String,
}

/// `ClickPipeKafkaSchemaRegistryCredentials` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistryCredentials {
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub username: String,
}

/// `ClickPipeKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSource {
    #[serde(default)]
    pub authentication: ClickPipeKafkaSourceAuthentication,
    #[serde(default)]
    pub brokers: String,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none", default)]
    pub consumer_group: Option<String>,
    #[serde(default)]
    pub format: ClickPipeKafkaSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<ClickPipeKafkaOffset>,
    #[serde(rename = "reversePrivateEndpointIds", default)]
    pub reverse_private_endpoint_ids: Vec<String>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none", default)]
    pub schema_registry: Option<ClickPipeKafkaSchemaRegistry>,
    #[serde(default)]
    pub topics: String,
    #[serde(default)]
    pub r#type: ClickPipeKafkaSourceType,
}

/// `ClickPipeKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKinesisSource {
    #[serde(default)]
    pub authentication: ClickPipeKinesisSourceAuthentication,
    #[serde(default)]
    pub format: ClickPipeKinesisSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType", default)]
    pub iterator_type: ClickPipeKinesisSourceIteratortype,
    #[serde(default)]
    pub region: String,
    #[serde(rename = "streamName", default)]
    pub stream_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none", default)]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettings {
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none", default)]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMongoDBPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(rename = "snapshotNumberOfParallelTables", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useJsonNativeFormat", skip_serializing_if = "Option::is_none", default)]
    pub use_json_native_format: Option<bool>,
}

/// `ClickPipeMongoDBPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: String,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
}

/// `ClickPipeMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipeMongoDBSourceReadpreference,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeMongoDBPipeSettings>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings: Option<Vec<ClickPipeMongoDBPipeTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    pub uri: String,
}

/// `ClickPipeMutateBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateBigQuerySource {
    #[serde(default)]
    pub credentials: ServiceAccount,
    #[serde(default)]
    pub settings: ClickPipeBigQueryPipeSettings,
    #[serde(rename = "snapshotStagingPath", default)]
    pub snapshot_staging_path: String,
    #[serde(rename = "tableMappings", default)]
    pub table_mappings: Vec<ClickPipeBigQueryPipeTableMapping>,
}

/// `ClickPipeMutateDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateDestination {
    #[serde(default)]
    pub columns: Vec<ClickPipeDestinationColumn>,
    #[serde(default)]
    pub database: String,
    #[serde(rename = "managedTable", default)]
    pub managed_table: bool,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub table: String,
    #[serde(rename = "tableDefinition", default)]
    pub table_definition: ClickPipeDestinationTableDefinition,
}

/// `ClickPipeMutateKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateKafkaSchemaRegistry {
    #[serde(default)]
    pub authentication: ClickPipeMutateKafkaSchemaRegistryAuthentication,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(default)]
    pub credentials: ClickPipeKafkaSchemaRegistryCredentials,
    #[serde(default)]
    pub url: String,
}

/// `ClickPipeMutateMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipeMutateMongoDBSourceReadpreference,
    pub settings: ClickPipeMongoDBPipeSettings,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMongoDBPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    pub uri: String,
}

/// `ClickPipeMutateMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeMutateMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    pub host: String,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    pub port: i64,
    pub settings: ClickPipeMySQLPipeSettings,
    #[serde(rename = "skipCertVerification", skip_serializing_if = "Option::is_none", default)]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMySQLPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeMutateMySQLSourceType>,
}

/// `ClickPipeMutatePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutatePostgresSource {
    #[serde(default)]
    pub authentication: ClickPipeMutatePostgresSourceAuthentication,
    #[serde(rename = "caCertificate", default)]
    pub ca_certificate: String,
    #[serde(default)]
    pub credentials: PLAIN,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub host: String,
    #[serde(rename = "iamRole", default)]
    pub iam_role: String,
    #[serde(default)]
    pub port: i64,
    #[serde(default)]
    pub settings: ClickPipePostgresPipeSettings,
    #[serde(rename = "tableMappings", default)]
    pub table_mappings: Vec<ClickPipePostgresPipeTableMapping>,
    #[serde(rename = "tlsHost", default)]
    pub tls_host: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeMutatePostgresSourceType>,
}

/// `ClickPipeMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeSettings {
    #[serde(rename = "allowNullableColumns", skip_serializing_if = "Option::is_none", default)]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none", default)]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "initialLoadParallelism", skip_serializing_if = "Option::is_none", default)]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMechanism", skip_serializing_if = "Option::is_none", default)]
    pub replication_mechanism: Option<ClickPipeMySQLPipeSettingsReplicationmechanism>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMySQLPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(rename = "snapshotNumberOfParallelTables", skip_serializing_if = "Option::is_none", default)]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none", default)]
    pub use_compression: Option<bool>,
}

/// `ClickPipeMySQLPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none", default)]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none", default)]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipeMySQLPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey", skip_serializing_if = "Option::is_none", default)]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    pub host: String,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    pub port: i64,
    pub settings: ClickPipeMySQLPipeSettings,
    #[serde(rename = "skipCertVerification", skip_serializing_if = "Option::is_none", default)]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMySQLPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipeMySQLSourceType>,
}

/// `ClickPipeObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeObjectStorageSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipeObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none", default)]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compression: Option<ClickPipeObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub delimiter: Option<String>,
    #[serde(default)]
    pub format: ClickPipeObjectStorageSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none", default)]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none", default)]
    pub queue_url: Option<String>,
    #[serde(default)]
    pub r#type: ClickPipeObjectStorageSourceType,
    #[serde(default)]
    pub url: String,
}

/// `ClickPipePatchDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchDestination {
    #[serde(default)]
    pub columns: Vec<ClickPipeDestinationColumn>,
}

/// `ClickPipePatchKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchKafkaSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(default)]
    pub credentials: serde_json::Value,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "reversePrivateEndpointIds", default)]
    pub reverse_private_endpoint_ids: Vec<String>,
}

/// `ClickPipePatchKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchKinesisSourceAuthentication>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
}

/// `ClickPipePatchMongoDBPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeRemoveTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePatchMongoDBPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference", skip_serializing_if = "Option::is_none", default)]
    pub read_preference: Option<ClickPipePatchMongoDBSourceReadpreference>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePatchMongoDBPipeSettings>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_add: Option<Vec<ClickPipeMongoDBPipeTableMapping>>,
    #[serde(rename = "tableMappingsToRemove", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMongoDBPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
    pub uri: Option<String>,
}

/// `ClickPipePatchMySQLPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeRemoveTableMapping {
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePatchMySQLPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none", default)]
    pub use_compression: Option<bool>,
}

/// `ClickPipePatchMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none", default)]
    pub disable_tls: Option<bool>,
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipePatchMySQLPipeSettings>,
    #[serde(rename = "skipCertVerification", skip_serializing_if = "Option::is_none", default)]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_add: Option<Vec<ClickPipeMySQLPipeTableMapping>>,
    #[serde(rename = "tableMappingsToRemove", skip_serializing_if = "Option::is_none", default)]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMySQLPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePatchObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none", default)]
    pub azure_container_name: Option<String>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none", default)]
    pub service_account_key: Option<String>,
}

/// `ClickPipePatchPostgresPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeRemoveTableMapping {
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none", default)]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none", default)]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none", default)]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none", default)]
    pub table_engine: Option<ClickPipePatchPostgresPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none", default)]
    pub target_table: Option<String>,
}

/// `ClickPipePatchPostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none", default)]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "syncIntervalSeconds", skip_serializing_if = "Option::is_none", default)]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchPostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(default)]
    pub credentials: PLAIN,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub database: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub settings: ClickPipePatchPostgresPipeSettings,
    #[serde(rename = "tableMappingsToAdd", default)]
    pub table_mappings_to_add: Vec<ClickPipePostgresPipeTableMapping>,
    #[serde(rename = "tableMappingsToRemove", default)]
    pub table_mappings_to_remove: Vec<ClickPipePatchPostgresPipeRemoveTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none", default)]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub destination: Option<ClickPipePatchDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none", default)]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub settings: Option<ClickPipeSettings>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ClickPipePatchSource>,
}

/// `ClickPipePatchSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kafka: Option<ClickPipePatchKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kinesis: Option<ClickPipePatchKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mongodb: Option<ClickPipePatchMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mysql: Option<ClickPipePatchMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none", default)]
    pub object_storage: Option<ClickPipePatchObjectStorageSource>,
    #[serde(default)]
    pub postgres: ClickPipePatchPostgresSource,
    #[serde(rename = "validateSamples", default)]
    pub validate_samples: bool,
}

/// `ClickPipePostKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKafkaSource {
    #[serde(default)]
    pub authentication: ClickPipePostKafkaSourceAuthentication,
    #[serde(default)]
    pub brokers: String,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none", default)]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none", default)]
    pub consumer_group: Option<String>,
    #[serde(default)]
    pub credentials: serde_json::Value,
    #[serde(default)]
    pub format: ClickPipePostKafkaSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<ClickPipeKafkaOffset>,
    #[serde(rename = "reversePrivateEndpointIds", default)]
    pub reverse_private_endpoint_ids: Vec<String>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none", default)]
    pub schema_registry: Option<ClickPipeMutateKafkaSchemaRegistry>,
    #[serde(default)]
    pub topics: String,
    #[serde(default)]
    pub r#type: ClickPipePostKafkaSourceType,
}

/// `ClickPipePostKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(default)]
    pub authentication: ClickPipePostKinesisSourceAuthentication,
    #[serde(default)]
    pub format: ClickPipePostKinesisSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType", default)]
    pub iterator_type: ClickPipePostKinesisSourceIteratortype,
    #[serde(default)]
    pub region: String,
    #[serde(rename = "streamName", default)]
    pub stream_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none", default)]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipePostObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none", default)]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authentication: Option<ClickPipePostObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none", default)]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compression: Option<ClickPipePostObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none", default)]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub delimiter: Option<String>,
    #[serde(default)]
    pub format: ClickPipePostObjectStorageSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none", default)]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none", default)]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none", default)]
    pub queue_url: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none", default)]
    pub service_account_key: Option<String>,
    #[serde(default)]
    pub r#type: ClickPipePostObjectStorageSourceType,
    #[serde(default)]
    pub url: String,
}

/// `ClickPipePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostRequest {
    #[serde(default)]
    pub destination: ClickPipeMutateDestination,
    #[serde(rename = "fieldMappings", default)]
    pub field_mappings: Vec<ClickPipeFieldMapping>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub scaling: ClickPipeScaling,
    #[serde(default)]
    pub settings: ClickPipeSettings,
    #[serde(default)]
    pub source: ClickPipePostSource,
}

/// `ClickPipePostSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bigquery: Option<ClickPipeMutateBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kafka: Option<ClickPipePostKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kinesis: Option<ClickPipePostKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mongodb: Option<ClickPipeMutateMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mysql: Option<ClickPipeMutateMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none", default)]
    pub object_storage: Option<ClickPipePostObjectStorageSource>,
    #[serde(default)]
    pub postgres: ClickPipeMutatePostgresSource,
    #[serde(rename = "validateSamples", default)]
    pub validate_samples: bool,
}

/// `ClickPipePostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeSettings {
    #[serde(rename = "allowNullableColumns", default)]
    pub allow_nullable_columns: bool,
    #[serde(rename = "deleteOnMerge", default)]
    pub delete_on_merge: bool,
    #[serde(rename = "enableFailoverSlots", default)]
    pub enable_failover_slots: bool,
    #[serde(rename = "initialLoadParallelism", default)]
    pub initial_load_parallelism: i64,
    #[serde(rename = "publicationName", default)]
    pub publication_name: String,
    #[serde(rename = "pullBatchSize", default)]
    pub pull_batch_size: i64,
    #[serde(rename = "replicationMode", default)]
    pub replication_mode: ClickPipePostgresPipeSettingsReplicationmode,
    #[serde(rename = "replicationSlotName", default)]
    pub replication_slot_name: String,
    #[serde(rename = "snapshotNumRowsPerPartition", default)]
    pub snapshot_num_rows_per_partition: i64,
    #[serde(rename = "snapshotNumberOfParallelTables", default)]
    pub snapshot_number_of_parallel_tables: i64,
    #[serde(rename = "syncIntervalSeconds", default)]
    pub sync_interval_seconds: i64,
}

/// `ClickPipePostgresPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeTableMapping {
    #[serde(rename = "excludedColumns", default)]
    pub excluded_columns: Vec<String>,
    #[serde(rename = "partitionKey", default)]
    pub partition_key: String,
    #[serde(rename = "sortingKeys", default)]
    pub sorting_keys: Vec<String>,
    #[serde(rename = "sourceSchemaName", default)]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable", default)]
    pub source_table: String,
    #[serde(rename = "tableEngine", default)]
    pub table_engine: ClickPipePostgresPipeTableMappingTableengine,
    #[serde(rename = "targetTable", default)]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey", default)]
    pub use_custom_sorting_key: bool,
}

/// `ClickPipePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresSource {
    #[serde(default)]
    pub authentication: ClickPipePostgresSourceAuthentication,
    #[serde(rename = "caCertificate", default)]
    pub ca_certificate: String,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub host: String,
    #[serde(rename = "iamRole", default)]
    pub iam_role: String,
    #[serde(default)]
    pub port: i64,
    #[serde(default)]
    pub settings: ClickPipePostgresPipeSettings,
    #[serde(rename = "tableMappings", default)]
    pub table_mappings: Vec<ClickPipePostgresPipeTableMapping>,
    #[serde(rename = "tlsHost", default)]
    pub tls_host: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickPipePostgresSourceType>,
}

/// `ClickPipeScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScaling {
    #[serde(default)]
    pub concurrency: i64,
    #[serde(rename = "replicaCpuMillicores", default)]
    pub replica_cpu_millicores: i64,
    #[serde(rename = "replicaMemoryGb", default)]
    pub replica_memory_gb: f64,
    #[serde(default)]
    pub replicas: i64,
}

/// `ClickPipeScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScalingPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub concurrency: Option<i64>,
    #[serde(rename = "replicaCpuMillicores", skip_serializing_if = "Option::is_none", default)]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub replicas: Option<i64>,
}

/// `ClickPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettings {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSettingsPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettingsPutRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSource {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bigquery: Option<ClickPipeBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kafka: Option<ClickPipeKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kinesis: Option<ClickPipeKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mongodb: Option<ClickPipeMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mysql: Option<ClickPipeMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none", default)]
    pub object_storage: Option<ClickPipeObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub postgres: Option<ClickPipePostgresSource>,
}

/// `ClickPipeStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command: Option<ClickPipeStatePatchRequestCommand>,
}

/// `ClickPipesCdcScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScaling {
    #[serde(rename = "replicaCpuMillicores", default)]
    pub replica_cpu_millicores: i64,
    #[serde(rename = "replicaMemoryGb", default)]
    pub replica_memory_gb: f64,
}

/// `ClickPipesCdcScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScalingPatchRequest {
    #[serde(rename = "replicaCpuMillicores", skip_serializing_if = "Option::is_none", default)]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub replica_memory_gb: Option<f64>,
}

/// `ClickStackAggregatedColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAggregatedColumn {
    #[serde(rename = "aggFn")]
    pub agg_fn: String,
    #[serde(rename = "mvColumn")]
    pub mv_column: String,
    #[serde(rename = "sourceColumn", skip_serializing_if = "Option::is_none", default)]
    pub source_column: Option<String>,
}

/// `ClickStackAlertChannelEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelEmail {
    #[serde(rename = "emailRecipients")]
    pub email_recipients: Vec<String>,
    pub r#type: ClickStackAlertChannelEmailType,
}

/// `ClickStackAlertChannelWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelWebhook {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub severity: Option<ClickStackAlertChannelWebhookSeverity>,
    #[serde(rename = "slackChannelId", skip_serializing_if = "Option::is_none", default)]
    pub slack_channel_id: Option<String>,
    pub r#type: ClickStackAlertChannelWebhookType,
    #[serde(rename = "webhookId")]
    pub webhook_id: String,
    #[serde(rename = "webhookService", skip_serializing_if = "Option::is_none", default)]
    pub webhook_service: Option<String>,
}

/// `ClickStackAlertResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertResponse {
    #[serde(default)]
    pub channel: ClickStackAlertChannel,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none", default)]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub interval: ClickStackAlertResponseInterval,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none", default)]
    pub saved_search_id: Option<String>,
    #[serde(rename = "scheduleOffsetMinutes", skip_serializing_if = "Option::is_none", default)]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none", default)]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub silenced: Option<ClickStackAlertSilenced>,
    #[serde(default)]
    pub source: ClickStackAlertResponseSource,
    #[serde(default)]
    pub state: ClickStackAlertResponseState,
    #[serde(rename = "teamId", default)]
    pub team_id: String,
    #[serde(default)]
    pub threshold: f64,
    #[serde(rename = "thresholdType", default)]
    pub threshold_type: ClickStackAlertResponseThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none", default)]
    pub tile_id: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackAlertSilenced` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertSilenced {
    #[serde(default)]
    pub at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub by: Option<String>,
    #[serde(default)]
    pub until: chrono::DateTime<chrono::Utc>,
}

/// `ClickStackBarBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarBuilderChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackBarRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarRawSqlChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackBarRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackCreateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateAlertRequest {
    #[serde(default)]
    pub channel: ClickStackAlertChannel,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none", default)]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(default)]
    pub interval: ClickStackCreateAlertRequestInterval,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none", default)]
    pub saved_search_id: Option<String>,
    #[serde(rename = "scheduleOffsetMinutes", skip_serializing_if = "Option::is_none", default)]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none", default)]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub source: ClickStackCreateAlertRequestSource,
    #[serde(default)]
    pub threshold: f64,
    #[serde(rename = "thresholdType", default)]
    pub threshold_type: ClickStackCreateAlertRequestThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none", default)]
    pub tile_id: Option<String>,
}

/// `ClickStackCreateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filters: Option<Vec<ClickStackFilterInput>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none", default)]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none", default)]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none", default)]
    pub saved_query_language: Option<ClickStackCreateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `ClickStackDashboardResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardResponse {
    #[serde(default)]
    pub filters: Vec<ClickStackFilter>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none", default)]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none", default)]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none", default)]
    pub saved_query_language: Option<ClickStackDashboardResponseSavedquerylanguage>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tiles: Vec<ClickStackTileOutput>,
}

/// `ClickStackFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilter {
    pub expression: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none", default)]
    pub source_metric_type: Option<ClickStackFilterSourcemetrictype>,
    pub r#type: ClickStackFilterType,
}

/// `ClickStackFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterInput {
    pub expression: String,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none", default)]
    pub source_metric_type: Option<ClickStackFilterInputSourcemetrictype>,
    pub r#type: ClickStackFilterInputType,
}

/// `ClickStackFilterSettingsColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterSettingsColumn {
    pub label: String,
    pub name: String,
}

/// `ClickStackGenericWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackGenericWebhook {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub body: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackGenericWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackHighlightedAttributeExpression` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHighlightedAttributeExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(rename = "luceneExpression", skip_serializing_if = "Option::is_none", default)]
    pub lucene_expression: Option<String>,
    #[serde(rename = "sqlExpression")]
    pub sql_expression: String,
}

/// `ClickStackIncidentIOWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackIncidentIOWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackIncidentIOWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackLineBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineBuilderChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(rename = "compareToPreviousPeriod", skip_serializing_if = "Option::is_none", default)]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackLineRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineRawSqlChartConfig {
    #[serde(rename = "alignDateRangeToGranularity", skip_serializing_if = "Option::is_none", default)]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "compareToPreviousPeriod", skip_serializing_if = "Option::is_none", default)]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackLineRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none", default)]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackLogSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSource {
    #[serde(rename = "bodyExpression", skip_serializing_if = "Option::is_none", default)]
    pub body_expression: Option<String>,
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression")]
    pub default_table_select_expression: String,
    #[serde(rename = "displayedTimestampValueExpression", skip_serializing_if = "Option::is_none", default)]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(rename = "eventAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none", default)]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(rename = "highlightedRowAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_row_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(rename = "highlightedTraceAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_trace_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    pub id: String,
    #[serde(rename = "implicitColumnExpression", skip_serializing_if = "Option::is_none", default)]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackLogSourceKind,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none", default)]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none", default)]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub resource_attributes_expression: Option<String>,
    #[serde(rename = "serviceNameExpression", skip_serializing_if = "Option::is_none", default)]
    pub service_name_expression: Option<String>,
    #[serde(rename = "severityTextExpression", skip_serializing_if = "Option::is_none", default)]
    pub severity_text_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none", default)]
    pub span_id_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none", default)]
    pub trace_id_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none", default)]
    pub trace_source_id: Option<String>,
}

/// `ClickStackMarkdownChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackMarkdownChartConfigDisplaytype,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub markdown: Option<String>,
}

/// `ClickStackMarkdownChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartSeries {
    pub content: String,
    pub r#type: ClickStackMarkdownChartSeriesType,
}

/// `ClickStackMaterializedView` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMaterializedView {
    #[serde(rename = "aggregatedColumns")]
    pub aggregated_columns: Vec<ClickStackAggregatedColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "dimensionColumns")]
    pub dimension_columns: String,
    #[serde(rename = "minDate", skip_serializing_if = "Option::is_none", default)]
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "minGranularity")]
    pub min_granularity: ClickStackMaterializedViewMingranularity,
    #[serde(rename = "tableName")]
    pub table_name: String,
    #[serde(rename = "timestampColumn")]
    pub timestamp_column: String,
}

/// `ClickStackMetricSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSource {
    pub connection: String,
    pub from: ClickStackMetricSourceFrom,
    pub id: String,
    pub kind: ClickStackMetricSourceKind,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none", default)]
    pub log_source_id: Option<String>,
    #[serde(rename = "metricTables")]
    pub metric_tables: ClickStackMetricTables,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression")]
    pub resource_attributes_expression: String,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
}

/// `ClickStackMetricSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none", default)]
    pub table_name: Option<String>,
}

/// `ClickStackMetricTables` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricTables {
    #[serde(rename = "exponential histogram", default)]
    pub exponential_histogram: String,
    #[serde(default)]
    pub gauge: String,
    #[serde(default)]
    pub histogram: String,
    #[serde(default)]
    pub sum: String,
    #[serde(default)]
    pub summary: String,
}

/// `ClickStackNumberBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberBuilderChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackNumberChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackNumberChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none", default)]
    pub metric_data_type: Option<ClickStackNumberChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackNumberChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackNumberChartSeriesWherelanguage,
}

/// `ClickStackNumberFormat` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberFormat {
    #[serde(default)]
    pub average: bool,
    #[serde(rename = "currencySymbol", default)]
    pub currency_symbol: String,
    #[serde(rename = "decimalBytes", default)]
    pub decimal_bytes: bool,
    #[serde(default)]
    pub factor: f64,
    #[serde(default)]
    pub mantissa: i64,
    #[serde(default)]
    pub output: ClickStackNumberFormatOutput,
    #[serde(rename = "thousandSeparated", default)]
    pub thousand_separated: bool,
    #[serde(default)]
    pub unit: String,
}

/// `ClickStackNumberRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackNumberRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackPagerDutyAPIWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPagerDutyAPIWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackPagerDutyAPIWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackPieBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackPieRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackPieRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackQuerySetting` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQuerySetting {
    pub setting: String,
    pub value: String,
}

/// `ClickStackSavedFilterValue` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedFilterValue {
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<ClickStackSavedFilterValueType>,
}

/// `ClickStackSearchChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackSearchChartConfigDisplaytype,
    pub select: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartConfigWherelanguage,
}

/// `ClickStackSearchChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartSeries {
    pub fields: Vec<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackSearchChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartSeriesWherelanguage,
}

/// `ClickStackSelectItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSelectItem {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackSelectItemAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<ClickStackSelectItemLevel>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none", default)]
    pub metric_type: Option<ClickStackSelectItemMetrictype>,
    #[serde(rename = "periodAggFn", skip_serializing_if = "Option::is_none", default)]
    pub period_agg_fn: Option<ClickStackSelectItemPeriodaggfn>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none", default)]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none", default)]
    pub where_language: Option<ClickStackSelectItemWherelanguage>,
}

/// `ClickStackSessionSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSessionSource {
    pub connection: String,
    pub from: ClickStackSourceFrom,
    pub id: String,
    pub kind: ClickStackSessionSourceKind,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "timestampValueExpression", skip_serializing_if = "Option::is_none", default)]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceSourceId")]
    pub trace_source_id: String,
}

/// `ClickStackSlackAPIWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackAPIWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackSlackAPIWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackSlackWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackWebhook {
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    pub id: String,
    pub name: String,
    pub service: ClickStackSlackWebhookService,
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub url: Option<String>,
}

/// `ClickStackSourceFilterSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterSettings {
    pub columns: Vec<ClickStackFilterSettingsColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackTableBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableBuilderChartConfig {
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub having: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none", default)]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackTableChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTableChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none", default)]
    pub metric_data_type: Option<ClickStackTableChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none", default)]
    pub sort_order: Option<ClickStackTableChartSeriesSortorder>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTableChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTableChartSeriesWherelanguage,
}

/// `ClickStackTableRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackTableRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackTileInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileInput {
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none", default)]
    pub as_ratio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<ClickStackTileConfig>,
    pub h: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub series: Option<Vec<ClickStackDashboardChartSeries>>,
    pub w: i64,
    pub x: i64,
    pub y: i64,
}

/// `ClickStackTileOutput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileOutput {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<ClickStackTileConfig>,
    pub h: i64,
    pub id: String,
    pub name: String,
    pub w: i64,
    pub x: i64,
    pub y: i64,
}

/// `ClickStackTimeChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTimeChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTimeChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none", default)]
    pub display_type: Option<ClickStackTimeChartSeriesDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none", default)]
    pub metric_data_type: Option<ClickStackTimeChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none", default)]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none", default)]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTimeChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTimeChartSeriesWherelanguage,
}

/// `ClickStackTraceSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSource {
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression", skip_serializing_if = "Option::is_none", default)]
    pub default_table_select_expression: Option<String>,
    #[serde(rename = "durationExpression")]
    pub duration_expression: String,
    #[serde(rename = "durationPrecision")]
    pub duration_precision: i64,
    #[serde(rename = "eventAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none", default)]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(rename = "highlightedRowAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_row_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(rename = "highlightedTraceAttributeExpressions", skip_serializing_if = "Option::is_none", default)]
    pub highlighted_trace_attribute_expressions: Option<Vec<ClickStackHighlightedAttributeExpression>>,
    pub id: String,
    #[serde(rename = "implicitColumnExpression", skip_serializing_if = "Option::is_none", default)]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackTraceSourceKind,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none", default)]
    pub log_source_id: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none", default)]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none", default)]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "parentSpanIdExpression")]
    pub parent_span_id_expression: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none", default)]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression", skip_serializing_if = "Option::is_none", default)]
    pub resource_attributes_expression: Option<String>,
    #[serde(rename = "serviceNameExpression", skip_serializing_if = "Option::is_none", default)]
    pub service_name_expression: Option<String>,
    #[serde(rename = "sessionSourceId", skip_serializing_if = "Option::is_none", default)]
    pub session_source_id: Option<String>,
    #[serde(rename = "spanEventsValueExpression", skip_serializing_if = "Option::is_none", default)]
    pub span_events_value_expression: Option<String>,
    #[serde(rename = "spanIdExpression")]
    pub span_id_expression: String,
    #[serde(rename = "spanKindExpression")]
    pub span_kind_expression: String,
    #[serde(rename = "spanNameExpression")]
    pub span_name_expression: String,
    #[serde(rename = "statusCodeExpression", skip_serializing_if = "Option::is_none", default)]
    pub status_code_expression: Option<String>,
    #[serde(rename = "statusMessageExpression", skip_serializing_if = "Option::is_none", default)]
    pub status_message_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression")]
    pub trace_id_expression: String,
}

/// `ClickStackUpdateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateAlertRequest {
    #[serde(default)]
    pub channel: ClickStackAlertChannel,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none", default)]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none", default)]
    pub group_by: Option<String>,
    #[serde(default)]
    pub interval: ClickStackUpdateAlertRequestInterval,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none", default)]
    pub saved_search_id: Option<String>,
    #[serde(rename = "scheduleOffsetMinutes", skip_serializing_if = "Option::is_none", default)]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none", default)]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub source: ClickStackUpdateAlertRequestSource,
    #[serde(default)]
    pub threshold: f64,
    #[serde(rename = "thresholdType", default)]
    pub threshold_type: ClickStackUpdateAlertRequestThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none", default)]
    pub tile_id: Option<String>,
}

/// `ClickStackUpdateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filters: Option<Vec<ClickStackFilter>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none", default)]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none", default)]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none", default)]
    pub saved_query_language: Option<ClickStackUpdateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `CreateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateReversePrivateEndpoint {
    #[serde(default)]
    pub description: String,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none", default)]
    pub msk_authentication: Option<CreateReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none", default)]
    pub msk_cluster_arn: Option<String>,
    #[serde(default)]
    pub r#type: CreateReversePrivateEndpointType,
    #[serde(rename = "vpcEndpointServiceName", skip_serializing_if = "Option::is_none", default)]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(rename = "vpcResourceConfigurationId", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(rename = "vpcResourceShareArn", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_share_arn: Option<String>,
}

/// `GcpBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucket {
    #[serde(rename = "accessKeyId", default)]
    pub access_key_id: String,
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: GcpBackupBucketBucketprovider,
    #[serde(default)]
    pub id: uuid::Uuid,
}

/// `GcpBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPatchRequestV1 {
    #[serde(rename = "accessKeyId", default)]
    pub access_key_id: String,
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: GcpBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "secretAccessKey", default)]
    pub secret_access_key: String,
}

/// `GcpBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPostRequestV1 {
    #[serde(rename = "accessKeyId", default)]
    pub access_key_id: String,
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: GcpBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "secretAccessKey", default)]
    pub secret_access_key: String,
}

/// `GcpBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketProperties {
    #[serde(rename = "accessKeyId", default)]
    pub access_key_id: String,
    #[serde(rename = "bucketPath", default)]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider", default)]
    pub bucket_provider: GcpBackupBucketPropertiesBucketprovider,
}

/// `InstancePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpoint {
    #[serde(rename = "cloudProvider", default)]
    pub cloud_provider: InstancePrivateEndpointCloudprovider,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub region: InstancePrivateEndpointRegion,
}

/// `InstancePrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpointsPatch {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

/// `InstanceServiceQueryApiEndpointsPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceServiceQueryApiEndpointsPostRequest {
    #[serde(rename = "allowedOrigins", default)]
    pub allowed_origins: String,
    #[serde(rename = "openApiKeys", default)]
    pub open_api_keys: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// `InstanceTagsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceTagsPatch {
    #[serde(default)]
    pub add: Vec<ResourceTagsV1>,
    #[serde(default)]
    pub remove: Vec<ResourceTagsV1>,
}

/// `Invitation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Invitation {
    #[serde(rename = "assignedRoles", default)]
    pub assigned_roles: Vec<AssignedRole>,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub email: String,
    #[serde(rename = "expireAt", default)]
    pub expire_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(default)]
    pub role: InvitationRole,
}

/// `InvitationPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InvitationPostRequest {
    #[serde(rename = "assignedRoleIds", default)]
    pub assigned_role_ids: Vec<String>,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub role: InvitationPostRequestRole,
}

/// `IpAccessListEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListEntry {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source: String,
}

/// `IpAccessListPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListPatch {
    #[serde(default)]
    pub add: Vec<IpAccessListEntry>,
    #[serde(default)]
    pub remove: Vec<IpAccessListEntry>,
}

/// `Member` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Member {
    #[serde(rename = "assignedRoles", default)]
    pub assigned_roles: Vec<AssignedRole>,
    #[serde(default)]
    pub email: String,
    #[serde(rename = "joinedAt", default)]
    pub joined_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: MemberRole,
    #[serde(rename = "userId", default)]
    pub user_id: String,
}

/// `MemberPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none", default)]
    pub assigned_role_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub role: Option<MemberPatchRequestRole>,
}

/// `MskIamUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MskIamUser {
    #[serde(rename = "accessKeyId", default)]
    pub access_key_id: String,
    #[serde(rename = "secretKey", default)]
    pub secret_key: String,
}

/// `MutualTLS` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MutualTLS {
    #[serde(default)]
    pub certificate: String,
    #[serde(rename = "privateKey", default)]
    pub private_key: String,
}

/// `Organization` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Organization {
    #[serde(rename = "byocConfig", default)]
    pub byoc_config: Vec<ByocConfig>,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "enableCoreDumps", default)]
    pub enable_core_dumps: bool,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "privateEndpoints", default)]
    pub private_endpoints: Vec<OrganizationPrivateEndpoint>,
}

/// `OrganizationCloudRegionPrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationCloudRegionPrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", default)]
    pub endpoint_service_id: String,
}

/// `OrganizationPatchPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchPrivateEndpoint {
    #[serde(rename = "cloudProvider", default)]
    pub cloud_provider: OrganizationPatchPrivateEndpointCloudprovider,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub region: OrganizationPatchPrivateEndpointRegion,
}

/// `OrganizationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoints: Option<OrganizationPrivateEndpointsPatch>,
}

/// `OrganizationPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpoint {
    #[serde(rename = "cloudProvider", default)]
    pub cloud_provider: OrganizationPrivateEndpointCloudprovider,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub region: OrganizationPrivateEndpointRegion,
}

/// `OrganizationPrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpointsPatch {
    #[serde(default)]
    pub add: Vec<OrganizationPatchPrivateEndpoint>,
    #[serde(default)]
    pub remove: Vec<OrganizationPatchPrivateEndpoint>,
}

/// `PLAIN` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PLAIN {
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub username: String,
}

/// `PostgresService` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresService {
    #[serde(rename = "connectionString", default)]
    pub connection_string: String,
    #[serde(rename = "createdAt", default)]
    pub created_at: PgCreatedAtProperty,
    #[serde(rename = "haType", default)]
    pub ha_type: PgHaType,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub id: PgIdProperty,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: PgIsPrimaryProperty,
    #[serde(default)]
    pub name: PgNameProperty,
    #[serde(default)]
    pub password: String,
    #[serde(rename = "postgresVersion", default)]
    pub postgres_version: PgVersion,
    #[serde(default)]
    pub provider: PgProvider,
    #[serde(default)]
    pub region: PgRegion,
    #[serde(default)]
    pub size: PgSize,
    #[serde(default)]
    pub state: PgStateProperty,
    #[serde(rename = "storageSize", default)]
    pub storage_size: PgStorageSize,
    #[serde(default)]
    pub tags: PgTags,
    #[serde(default)]
    pub username: String,
}

/// `PostgresServiceListItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceListItem {
    #[serde(rename = "createdAt", default)]
    pub created_at: PgCreatedAtProperty,
    #[serde(rename = "haType", default)]
    pub ha_type: PgHaType,
    #[serde(default)]
    pub id: PgIdProperty,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: PgIsPrimaryProperty,
    #[serde(default)]
    pub name: PgNameProperty,
    #[serde(rename = "postgresVersion", default)]
    pub postgres_version: PgVersion,
    #[serde(default)]
    pub provider: PgProvider,
    #[serde(default)]
    pub region: PgRegion,
    #[serde(default)]
    pub size: PgSize,
    #[serde(default)]
    pub state: PgStateProperty,
    #[serde(rename = "storageSize", default)]
    pub storage_size: PgStorageSize,
    #[serde(default)]
    pub tags: PgTags,
}

/// `PostgresServicePasswordResource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePasswordResource {
    #[serde(default)]
    pub password: String,
}

/// `PostgresServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePatchRequest {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<PgNameProperty>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<PgSize>,
    #[serde(rename = "storageSize", skip_serializing_if = "Option::is_none", default)]
    pub storage_size: Option<PgStorageSize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePostRequest {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none", default)]
    pub ha_type: Option<PgHaType>,
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_config: Option<PgConfig>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none", default)]
    pub postgres_version: Option<PgVersion>,
    pub provider: PgProvider,
    pub region: PgRegion,
    pub size: PgSize,
    #[serde(rename = "storageSize")]
    pub storage_size: PgStorageSize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceReadReplicaRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceReadReplicaRequest {
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_config: Option<PgConfig>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceRestoreRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceRestoreRequest {
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none", default)]
    pub pg_config: Option<PgConfig>,
    #[serde(rename = "restoreTarget")]
    pub restore_target: PgPitrRestoreTargetProperty,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceSetPassword` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceSetPassword {
    #[serde(default)]
    pub password: PgPassword,
}

/// `PostgresServiceSetState` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceSetState {
    #[serde(default)]
    pub command: PostgresServiceSetStateCommand,
}

/// `PrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", default)]
    pub endpoint_service_id: String,
    #[serde(rename = "privateDnsHostname", default)]
    pub private_dns_hostname: String,
}

/// `RBACPolicy` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicy {
    #[serde(rename = "allowDeny", default)]
    pub allow_deny: RBACPolicyAllowdeny,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(rename = "roleId", default)]
    pub role_id: String,
    #[serde(default)]
    pub tags: RBACPolicyTags,
    #[serde(rename = "tenantId", default)]
    pub tenant_id: String,
}

/// `RBACPolicyCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyCreateRequest {
    #[serde(rename = "allowDeny")]
    pub allow_deny: RBACPolicyCreateRequestAllowdeny,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<RBACPolicyTags>,
}

/// `RBACPolicyTags` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyTags {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub grants: Option<Vec<String>>,
    #[serde(rename = "roleV2", skip_serializing_if = "Option::is_none", default)]
    pub role_v2: Option<RBACPolicyTagsRolev2>,
}

/// `RBACRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACRole {
    #[serde(default)]
    pub actors: Vec<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "ownerId", default)]
    pub owner_id: String,
    #[serde(default)]
    pub policies: Vec<RBACPolicy>,
    #[serde(rename = "tenantId", default)]
    pub tenant_id: String,
    #[serde(default)]
    pub r#type: RBACRoleType,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// `ResourceTagsV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceTagsV1 {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpoint {
    #[serde(default)]
    pub description: String,
    #[serde(rename = "dnsNames", default)]
    pub dns_names: Vec<String>,
    #[serde(rename = "endpointId", default)]
    pub endpoint_id: String,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none", default)]
    pub msk_authentication: Option<ReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none", default)]
    pub msk_cluster_arn: Option<String>,
    #[serde(rename = "privateDnsNames", default)]
    pub private_dns_names: Vec<String>,
    #[serde(rename = "serviceId", default)]
    pub service_id: uuid::Uuid,
    #[serde(default)]
    pub status: ReversePrivateEndpointStatus,
    #[serde(default)]
    pub r#type: ReversePrivateEndpointType,
    #[serde(rename = "vpcEndpointServiceName", skip_serializing_if = "Option::is_none", default)]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(rename = "vpcResourceConfigurationId", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(rename = "vpcResourceShareArn", skip_serializing_if = "Option::is_none", default)]
    pub vpc_resource_share_arn: Option<String>,
}

/// `RoleCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleCreateRequest {
    pub actors: Vec<String>,
    pub name: String,
    pub policies: Vec<RBACPolicyCreateRequest>,
}

/// `RoleUpdateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    #[serde(default)]
    pub actors: Vec<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub policies: Vec<RBACPolicyCreateRequest>,
}

/// `ScimEnterpriseManager` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimEnterpriseManager {
    #[serde(rename = "$ref", default)]
    pub r#ref: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimEnterpriseUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimEnterpriseUser {
    #[serde(rename = "costCenter", default)]
    pub cost_center: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub division: String,
    #[serde(rename = "employeeNumber", default)]
    pub employee_number: String,
    #[serde(default)]
    pub manager: ScimEnterpriseManager,
    #[serde(default)]
    pub organization: String,
}

/// `ScimGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroup {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    pub id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub members: Option<Vec<ScimGroupMember>>,
    pub meta: ScimGroupMeta,
    pub schemas: Vec<String>,
}

/// `ScimGroupListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimGroup>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimGroupMember` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupMember {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    pub value: String,
}

/// `ScimGroupMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupMeta {
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimGroupPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupPostRequest {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub members: Option<Vec<ScimGroupMember>>,
    pub schemas: Vec<String>,
}

/// `ScimGroupPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupPutRequest {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub members: Option<Vec<ScimGroupMember>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<ScimGroupMeta>,
    pub schemas: Vec<String>,
}

/// `ScimListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimUser>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimPatchOp` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimPatchOp {
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimPatchOperation>,
    pub schemas: Vec<String>,
}

/// `ScimPatchOperation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimPatchOperation {
    pub op: ScimPatchOperationOp,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<String>,
}

/// `ScimUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUser {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<Vec<ScimUserGroup>>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    pub meta: ScimUserMeta,
    pub name: ScimUserName,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none", default)]
    pub nick_name: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none", default)]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none", default)]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none", default)]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none", default)]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none", default)]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserAddress` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserAddress {
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub formatted: String,
    #[serde(default)]
    pub locality: String,
    #[serde(rename = "postalCode", default)]
    pub postal_code: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub region: String,
    #[serde(rename = "streetAddress", default)]
    pub street_address: String,
    #[serde(default)]
    pub r#type: String,
}

/// `ScimUserEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserEmail {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#type: Option<String>,
    pub value: String,
}

/// `ScimUserEntitlement` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserEntitlement {
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimUserGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserGroup {
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimUserIm` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserIm {
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimUserMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserMeta {
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimUserName` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserName {
    #[serde(rename = "familyName", default)]
    pub family_name: String,
    #[serde(default)]
    pub formatted: String,
    #[serde(rename = "givenName", default)]
    pub given_name: String,
    #[serde(rename = "honorificPrefix", default)]
    pub honorific_prefix: String,
    #[serde(rename = "honorificSuffix", default)]
    pub honorific_suffix: String,
    #[serde(rename = "middleName", default)]
    pub middle_name: String,
}

/// `ScimUserPhoneNumber` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPhoneNumber {
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimUserPhoto` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPhoto {
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimUserPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPostRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<Vec<ScimUserGroup>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<ScimUserName>,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none", default)]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none", default)]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none", default)]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none", default)]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User", skip_serializing_if = "Option::is_none", default)]
    pub urn_ietf_params_scim_schemas_extension_enterprise_2_0_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none", default)]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none", default)]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPutRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none", default)]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none", default)]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub groups: Option<Vec<ScimUserGroup>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<ScimUserMeta>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<ScimUserName>,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none", default)]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub password: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none", default)]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none", default)]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none", default)]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    #[serde(rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User", skip_serializing_if = "Option::is_none", default)]
    pub urn_ietf_params_scim_schemas_extension_enterprise_2_0_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none", default)]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none", default)]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserRole {
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub value: String,
}

/// `ScimX509Certificate` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimX509Certificate {
    #[serde(default)]
    pub value: String,
}

/// `ScimAuthenticationScheme` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimAuthenticationScheme {
    pub description: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary: Option<bool>,
    #[serde(rename = "specUri", skip_serializing_if = "Option::is_none", default)]
    pub spec_uri: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// `ScimBooleanFeature` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimBooleanFeature {
    pub supported: bool,
}

/// `ScimResourceType` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimResourceType {
    pub description: String,
    pub endpoint: String,
    pub id: String,
    pub meta: ScimResourceTypeMeta,
    pub name: String,
    pub schema: String,
    #[serde(rename = "schemaExtensions")]
    pub schema_extensions: Vec<ScimSchemaExtension>,
    pub schemas: Vec<String>,
}

/// `ScimResourceTypeListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimResourceTypeListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimResourceType>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimResourceTypeMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimResourceTypeMeta {
    pub location: String,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimSchema` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchema {
    pub attributes: Vec<ScimSchemaAttribute>,
    pub description: String,
    pub id: String,
    pub meta: ScimSchemaMeta,
    pub name: String,
    pub schemas: Vec<String>,
}

/// `ScimSchemaAttribute` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaAttribute {
    #[serde(rename = "canonicalValues", skip_serializing_if = "Option::is_none", default)]
    pub canonical_values: Option<Vec<String>>,
    #[serde(rename = "caseExact", skip_serializing_if = "Option::is_none", default)]
    pub case_exact: Option<bool>,
    pub description: String,
    #[serde(rename = "multiValued")]
    pub multi_valued: bool,
    pub mutability: String,
    pub name: String,
    #[serde(rename = "referenceTypes", skip_serializing_if = "Option::is_none", default)]
    pub reference_types: Option<Vec<String>>,
    pub required: bool,
    pub returned: String,
    #[serde(rename = "subAttributes", skip_serializing_if = "Option::is_none", default)]
    pub sub_attributes: Option<Vec<ScimSchemaAttribute>>,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uniqueness: Option<String>,
}

/// `ScimSchemaExtension` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaExtension {
    pub required: bool,
    pub schema: String,
}

/// `ScimSchemaListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimSchema>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimSchemaMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaMeta {
    pub location: String,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimServiceProviderConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfig {
    #[serde(rename = "authenticationSchemes")]
    pub authentication_schemes: Vec<ScimAuthenticationScheme>,
    pub bulk: ScimServiceProviderConfigBulk,
    #[serde(rename = "changePassword")]
    pub change_password: ScimBooleanFeature,
    #[serde(rename = "documentationUri", skip_serializing_if = "Option::is_none", default)]
    pub documentation_uri: Option<String>,
    pub etag: ScimBooleanFeature,
    pub filter: ScimServiceProviderConfigFilter,
    pub meta: ScimServiceProviderConfigMeta,
    pub patch: ScimServiceProviderConfigPatch,
    pub schemas: Vec<String>,
    pub sort: ScimBooleanFeature,
}

/// `ScimServiceProviderConfigBulk` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigBulk {
    #[serde(rename = "maxOperations")]
    pub max_operations: i64,
    #[serde(rename = "maxPayloadSize")]
    pub max_payload_size: i64,
    pub supported: bool,
}

/// `ScimServiceProviderConfigFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigFilter {
    #[serde(rename = "maxResults")]
    pub max_results: i64,
    pub supported: bool,
}

/// `ScimServiceProviderConfigMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigMeta {
    pub location: String,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimServiceProviderConfigPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigPatch {
    pub supported: bool,
}

/// `ServicPrivateEndpointePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicPrivateEndpointePostRequest {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub id: String,
}

/// `Service` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Service {
    #[serde(rename = "availablePrivateEndpointIds", default)]
    pub available_private_endpoint_ids: Vec<String>,
    #[serde(rename = "byocId", default)]
    pub byoc_id: String,
    #[serde(rename = "clickhouseVersion", default)]
    pub clickhouse_version: String,
    #[serde(rename = "complianceType", default)]
    pub compliance_type: ServiceCompliancetype,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "dataWarehouseId", default)]
    pub data_warehouse_id: String,
    #[serde(rename = "enableCoreDumps", default)]
    pub enable_core_dumps: bool,
    #[serde(rename = "encryptionAssumedRoleIdentifier", skip_serializing_if = "Option::is_none", default)]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none", default)]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", default)]
    pub encryption_role_id: String,
    #[serde(default)]
    pub endpoints: Vec<ServiceEndpoint>,
    #[serde(rename = "hasTransparentDataEncryption", default)]
    pub has_transparent_data_encryption: bool,
    #[serde(rename = "iamRole", default)]
    pub iam_role: String,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(rename = "idleScaling", default)]
    pub idle_scaling: bool,
    #[serde(rename = "idleTimeoutMinutes", default)]
    pub idle_timeout_minutes: f64,
    #[serde(rename = "ipAccessList", default)]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
    #[serde(rename = "isReadonly", default)]
    pub is_readonly: bool,
    #[serde(rename = "maxReplicaMemoryGb", default)]
    pub max_replica_memory_gb: f64,
    #[serde(rename = "maxTotalMemoryGb", default)]
    pub max_total_memory_gb: f64,
    #[serde(rename = "minReplicaMemoryGb", default)]
    pub min_replica_memory_gb: f64,
    #[serde(rename = "minTotalMemoryGb", default)]
    pub min_total_memory_gb: f64,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "numReplicas", default)]
    pub num_replicas: f64,
    #[serde(rename = "privateEndpointIds", default)]
    pub private_endpoint_ids: Vec<String>,
    #[serde(default)]
    pub profile: ServiceProfile,
    #[serde(default)]
    pub provider: ServiceProvider,
    #[serde(default)]
    pub region: ServiceRegion,
    #[serde(rename = "releaseChannel", default)]
    pub release_channel: ServiceReleasechannel,
    #[serde(default)]
    pub state: ServiceState,
    #[serde(default)]
    pub tags: Vec<ResourceTagsV1>,
    #[serde(default)]
    pub tier: ServiceTier,
    #[serde(rename = "transparentDataEncryptionKeyId", default)]
    pub transparent_data_encryption_key_id: String,
}

/// `ServiceAccount` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "serviceAccountFile", default)]
    pub service_account_file: String,
}

/// `ServiceEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: f64,
    #[serde(default)]
    pub protocol: ServiceEndpointProtocol,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
}

/// `ServiceEndpointChange` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpointChange {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: ServiceEndpointChangeProtocol,
}

/// `ServicePasswordPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchRequest {
    #[serde(rename = "newDoubleSha1Hash", skip_serializing_if = "Option::is_none", default)]
    pub new_double_sha1_hash: Option<String>,
    #[serde(rename = "newPasswordHash", skip_serializing_if = "Option::is_none", default)]
    pub new_password_hash: Option<String>,
}

/// `ServicePasswordPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchResponse {
    #[serde(default)]
    pub password: String,
}

/// `ServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none", default)]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none", default)]
    pub ip_access_list: Option<IpAccessListPatch>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none", default)]
    pub private_endpoint_ids: Option<InstancePrivateEndpointsPatch>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none", default)]
    pub release_channel: Option<ServicePatchRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<InstanceTagsPatch>,
    #[serde(rename = "transparentDataEncryptionKeyId", skip_serializing_if = "Option::is_none", default)]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostRequest {
    #[serde(rename = "backupId", skip_serializing_if = "Option::is_none", default)]
    pub backup_id: Option<uuid::Uuid>,
    #[serde(rename = "byocId", default)]
    pub byoc_id: String,
    #[serde(rename = "complianceType", default)]
    pub compliance_type: ServicePostRequestCompliancetype,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none", default)]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", default)]
    pub enable_core_dumps: bool,
    #[serde(rename = "encryptionAssumedRoleIdentifier", skip_serializing_if = "Option::is_none", default)]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none", default)]
    pub encryption_key: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<ServiceEndpointChange>,
    #[serde(rename = "hasTransparentDataEncryption", default)]
    pub has_transparent_data_encryption: bool,
    #[serde(rename = "idleScaling", default)]
    pub idle_scaling: bool,
    #[serde(rename = "idleTimeoutMinutes", default)]
    pub idle_timeout_minutes: f64,
    #[serde(rename = "ipAccessList", default)]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(rename = "isReadonly", default)]
    pub is_readonly: bool,
    #[serde(rename = "maxReplicaMemoryGb", default)]
    pub max_replica_memory_gb: f64,
    #[serde(rename = "maxTotalMemoryGb", default)]
    pub max_total_memory_gb: f64,
    #[serde(rename = "minReplicaMemoryGb", default)]
    pub min_replica_memory_gb: f64,
    #[serde(rename = "minTotalMemoryGb", default)]
    pub min_total_memory_gb: f64,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "numReplicas", default)]
    pub num_replicas: f64,
    #[serde(rename = "privateEndpointIds", default)]
    pub private_endpoint_ids: Vec<String>,
    #[serde(rename = "privatePreviewTermsChecked", default)]
    pub private_preview_terms_checked: bool,
    #[serde(default)]
    pub profile: ServicePostRequestProfile,
    #[serde(default)]
    pub provider: ServicePostRequestProvider,
    #[serde(default)]
    pub region: ServicePostRequestRegion,
    #[serde(rename = "releaseChannel", default)]
    pub release_channel: ServicePostRequestReleasechannel,
    #[serde(default)]
    pub tags: Vec<ResourceTagsV1>,
    #[serde(default)]
    pub tier: ServicePostRequestTier,
}

/// `ServicePostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostResponse {
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub service: Service,
}

/// `ServiceQueryAPIEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceQueryAPIEndpoint {
    #[serde(rename = "allowedOrigins", default)]
    pub allowed_origins: String,
    #[serde(default)]
    pub id: String,
    #[serde(rename = "openApiKeys", default)]
    pub open_api_keys: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// `ServiceReplicaScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceReplicaScalingPatchRequest {
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
}

/// `ServiceScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchRequest {
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none", default)]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none", default)]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none", default)]
    pub min_total_memory_gb: Option<f64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none", default)]
    pub num_replicas: Option<f64>,
}

/// `ServiceScalingPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchResponse {
    #[serde(rename = "availablePrivateEndpointIds", default)]
    pub available_private_endpoint_ids: Vec<String>,
    #[serde(rename = "byocId", default)]
    pub byoc_id: String,
    #[serde(rename = "clickhouseVersion", default)]
    pub clickhouse_version: String,
    #[serde(rename = "complianceType", default)]
    pub compliance_type: ServiceScalingPatchResponseCompliancetype,
    #[serde(rename = "createdAt", default)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "dataWarehouseId", default)]
    pub data_warehouse_id: String,
    #[serde(rename = "enableCoreDumps", default)]
    pub enable_core_dumps: bool,
    #[serde(rename = "encryptionAssumedRoleIdentifier", skip_serializing_if = "Option::is_none", default)]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none", default)]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", default)]
    pub encryption_role_id: String,
    #[serde(default)]
    pub endpoints: Vec<ServiceEndpoint>,
    #[serde(rename = "hasTransparentDataEncryption", default)]
    pub has_transparent_data_encryption: bool,
    #[serde(rename = "iamRole", default)]
    pub iam_role: String,
    #[serde(default)]
    pub id: uuid::Uuid,
    #[serde(rename = "idleScaling", default)]
    pub idle_scaling: bool,
    #[serde(rename = "idleTimeoutMinutes", default)]
    pub idle_timeout_minutes: f64,
    #[serde(rename = "ipAccessList", default)]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(rename = "isPrimary", default)]
    pub is_primary: bool,
    #[serde(rename = "isReadonly", default)]
    pub is_readonly: bool,
    #[serde(rename = "maxReplicaMemoryGb", default)]
    pub max_replica_memory_gb: f64,
    #[serde(rename = "maxTotalMemoryGb", default)]
    pub max_total_memory_gb: f64,
    #[serde(rename = "minReplicaMemoryGb", default)]
    pub min_replica_memory_gb: f64,
    #[serde(rename = "minTotalMemoryGb", default)]
    pub min_total_memory_gb: f64,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "numReplicas", default)]
    pub num_replicas: f64,
    #[serde(rename = "privateEndpointIds", default)]
    pub private_endpoint_ids: Vec<String>,
    #[serde(default)]
    pub profile: ServiceScalingPatchResponseProfile,
    #[serde(default)]
    pub provider: ServiceScalingPatchResponseProvider,
    #[serde(default)]
    pub region: ServiceScalingPatchResponseRegion,
    #[serde(rename = "releaseChannel", default)]
    pub release_channel: ServiceScalingPatchResponseReleasechannel,
    #[serde(default)]
    pub state: ServiceScalingPatchResponseState,
    #[serde(default)]
    pub tags: Vec<ResourceTagsV1>,
    #[serde(default)]
    pub tier: ServiceScalingPatchResponseTier,
    #[serde(rename = "transparentDataEncryptionKeyId", default)]
    pub transparent_data_encryption_key_id: String,
}

/// `ServiceStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command: Option<ServiceStatePatchRequestCommand>,
}

/// `UsageCost` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCost {
    #[serde(default)]
    pub costs: Vec<UsageCostRecord>,
    #[serde(rename = "grandTotalCHC", default)]
    pub grand_total_chc: f64,
}

/// `UsageCostMetrics` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostMetrics {
    #[serde(rename = "backupCHC", default)]
    pub backup_chc: f64,
    #[serde(rename = "computeCHC", default)]
    pub compute_chc: f64,
    #[serde(rename = "dataTransferCHC", default)]
    pub data_transfer_chc: f64,
    #[serde(rename = "initialLoadCHC", default)]
    pub initial_load_chc: f64,
    #[serde(rename = "interRegionTier1DataTransferCHC", default)]
    pub inter_region_tier1_data_transfer_chc: f64,
    #[serde(rename = "interRegionTier2DataTransferCHC", default)]
    pub inter_region_tier2_data_transfer_chc: f64,
    #[serde(rename = "interRegionTier3DataTransferCHC", default)]
    pub inter_region_tier3_data_transfer_chc: f64,
    #[serde(rename = "interRegionTier4DataTransferCHC", default)]
    pub inter_region_tier4_data_transfer_chc: f64,
    #[serde(rename = "publicDataTransferCHC", default)]
    pub public_data_transfer_chc: f64,
    #[serde(rename = "storageCHC", default)]
    pub storage_chc: f64,
}

/// `UsageCostRecord` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostRecord {
    #[serde(rename = "dataWarehouseId", default)]
    pub data_warehouse_id: uuid::Uuid,
    #[serde(default)]
    pub date: String,
    #[serde(rename = "entityId", default)]
    pub entity_id: uuid::Uuid,
    #[serde(rename = "entityName", default)]
    pub entity_name: String,
    #[serde(rename = "entityType", default)]
    pub entity_type: UsageCostRecordEntitytype,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub metrics: UsageCostMetrics,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none", default)]
    pub service_id: Option<uuid::Uuid>,
    #[serde(rename = "totalCHC", default)]
    pub total_chc: f64,
}

/// `pgBouncerConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgBouncerConfig {
}

/// `pgConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgConfig {
    #[serde(default)]
    pub default_transaction_isolation: PgConfigDefaultTransactionIsolation,
    #[serde(default)]
    pub effective_cache_size: serde_json::Value,
    #[serde(default)]
    pub effective_io_concurrency: i64,
    #[serde(default)]
    pub idle_in_transaction_session_timeout: serde_json::Value,
    #[serde(default)]
    pub idle_session_timeout: serde_json::Value,
    #[serde(default)]
    pub lock_timeout: serde_json::Value,
    #[serde(default)]
    pub maintenance_work_mem: serde_json::Value,
    #[serde(default)]
    pub max_connections: i64,
    #[serde(default)]
    pub max_parallel_maintenance_workers: i64,
    #[serde(default)]
    pub max_parallel_workers: i64,
    #[serde(default)]
    pub max_parallel_workers_per_gather: i64,
    #[serde(default)]
    pub max_slot_wal_keep_size: serde_json::Value,
    #[serde(default)]
    pub max_wal_size: serde_json::Value,
    #[serde(default)]
    pub max_worker_processes: i64,
    #[serde(default)]
    pub min_wal_size: serde_json::Value,
    #[serde(default)]
    pub random_page_cost: f64,
    #[serde(default)]
    pub statement_timeout: serde_json::Value,
    #[serde(default)]
    pub transaction_timeout: serde_json::Value,
    #[serde(default)]
    pub wal_compression: PgConfigWalCompression,
    #[serde(default)]
    pub wal_keep_size: serde_json::Value,
    #[serde(default)]
    pub wal_sender_timeout: serde_json::Value,
    #[serde(default)]
    pub work_mem: serde_json::Value,
}

/// `postgresInstanceConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceConfig {
    #[serde(rename = "pgBouncerConfig")]
    pub pg_bouncer_config: PgBouncerConfig,
    #[serde(rename = "pgConfig")]
    pub pg_config: PgConfig,
}

/// `postgresInstanceUpdateConfigResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceUpdateConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
    #[serde(rename = "pgBouncerConfig")]
    pub pg_bouncer_config: PgBouncerConfig,
    #[serde(rename = "pgConfig")]
    pub pg_config: PgConfig,
}

/// Standard API response wrapper.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "requestId")]
    pub request_id: Option<String>,
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}


impl Default for BackupBucket {
    fn default() -> Self {
        Self::AwsBackupBucket(AwsBackupBucket::default())
    }
}


impl Default for BackupBucketPatchRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1::default())
    }
}


impl Default for BackupBucketPostRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1::default())
    }
}


impl Default for BackupBucketProperties {
    fn default() -> Self {
        Self::AwsBackupBucketProperties(AwsBackupBucketProperties::default())
    }
}


impl Default for ClickStackAlertChannel {
    fn default() -> Self {
        Self::ClickStackAlertChannelEmail(ClickStackAlertChannelEmail::default())
    }
}


impl Default for ClickStackBarChartConfig {
    fn default() -> Self {
        Self::ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig::default())
    }
}


impl Default for ClickStackDashboardChartSeries {
    fn default() -> Self {
        Self::ClickStackTimeChartSeries(ClickStackTimeChartSeries::default())
    }
}


impl Default for ClickStackLineChartConfig {
    fn default() -> Self {
        Self::ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig::default())
    }
}


impl Default for ClickStackNumberChartConfig {
    fn default() -> Self {
        Self::ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig::default())
    }
}


impl Default for ClickStackPieChartConfig {
    fn default() -> Self {
        Self::ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig::default())
    }
}


impl Default for ClickStackSource {
    fn default() -> Self {
        Self::ClickStackLogSource(ClickStackLogSource::default())
    }
}


impl Default for ClickStackTableChartConfig {
    fn default() -> Self {
        Self::ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig::default())
    }
}


impl Default for ClickStackTileConfig {
    fn default() -> Self {
        Self::ClickStackLineChartConfig(ClickStackLineChartConfig::default())
    }
}


impl Default for ClickStackWebhook {
    fn default() -> Self {
        Self::ClickStackSlackWebhook(ClickStackSlackWebhook::default())
    }
}
