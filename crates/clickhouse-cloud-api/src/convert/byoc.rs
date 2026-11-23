use super::MissingRequiredFields;
use crate::models::*;

impl TryFrom<ByocRestrictedFactsResponse> for ByocRestrictedFacts {
    type Error = MissingRequiredFields;

    /// Turns the restricted facts read from an infrastructure into ones that
    /// can be sent to register or validate it again.
    ///
    /// Facts without `backupBucket`, `clusterName`, `dataBucket`, `imageMode`
    /// or `storageRoleArn` cannot be written back: inventing a bucket, cluster
    /// or role would point the infrastructure at the wrong resource.
    fn try_from(value: ByocRestrictedFactsResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.backup_bucket.is_none() {
            missing.push("backupBucket");
        }
        if value.cluster_name.is_none() {
            missing.push("clusterName");
        }
        if value.data_bucket.is_none() {
            missing.push("dataBucket");
        }
        if value.image_mode.is_none() {
            missing.push("imageMode");
        }
        if value.storage_role_arn.is_none() {
            missing.push("storageRoleArn");
        }
        match (
            value.backup_bucket,
            value.cluster_name,
            value.data_bucket,
            value.image_mode,
            value.storage_role_arn,
        ) {
            (
                Some(backup_bucket),
                Some(cluster_name),
                Some(data_bucket),
                Some(image_mode),
                Some(storage_role_arn),
            ) => Ok(Self {
                availability_zones: value.availability_zones,
                backup_bucket,
                cluster_name,
                data_bucket,
                enable_autoscaler: value.enable_autoscaler,
                enable_monitoring: value.enable_monitoring,
                enable_snapshot_controller: value.enable_snapshot_controller,
                executor_role_arn: value.executor_role_arn,
                image_mode,
                image_registry: value.image_registry,
                namespace_prefix: value.namespace_prefix,
                node_role_arns: value.node_role_arns,
                platform_node_group: value.platform_node_group,
                scraper_endpoint: value.scraper_endpoint,
                storage_role_arn,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}
