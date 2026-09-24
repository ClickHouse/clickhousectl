use super::MissingRequiredFields;
use crate::models::{ClickPipeKinesisSchemaRegistry, ClickPipeKinesisSchemaRegistryResponse};

impl TryFrom<ClickPipeKinesisSchemaRegistryResponse> for ClickPipeKinesisSchemaRegistry {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickPipeKinesisSchemaRegistryResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.r#type.is_none() {
            missing.push("type");
        }
        if value.glue_region.is_none() {
            missing.push("glueRegion");
        }
        if value.glue_registry_name.is_none() {
            missing.push("glueRegistryName");
        }
        if !missing.is_empty() {
            return Err(MissingRequiredFields::new(missing));
        }

        Ok(Self {
            r#type: value.r#type.expect("checked above"),
            glue_region: value.glue_region.expect("checked above"),
            glue_registry_name: value.glue_registry_name.expect("checked above"),
            glue_role_arn: value.glue_role_arn,
        })
    }
}
