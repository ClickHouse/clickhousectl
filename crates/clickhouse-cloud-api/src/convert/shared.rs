use super::MissingRequiredFields;
use crate::models::*;

impl TryFrom<ResourceTagsV1Response> for ResourceTagsV1 {
    type Error = MissingRequiredFields;

    /// Turns a fetched tag into one that can be sent back.
    ///
    /// A tag is identified by its key, so a response tag without one cannot be
    /// written back — dropping it silently would delete the tag on the next
    /// write, and inventing an empty key would create a bogus one.
    fn try_from(value: ResourceTagsV1Response) -> Result<Self, Self::Error> {
        match value.key {
            Some(key) => Ok(Self {
                key,
                value: value.value,
            }),
            None => Err(MissingRequiredFields::new(vec!["key"])),
        }
    }
}
