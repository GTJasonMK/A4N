use super::errors::CollaborationDbError;
use serde::{Deserialize, Serialize};

pub(in crate::collaboration) const PROJECT_KIND_TEXT: &str = "text";
pub(in crate::collaboration) const PROJECT_KIND_COMIC: &str = "comic";
pub(in crate::collaboration) const PROJECT_KIND_VIDEO: &str = "video";
pub(in crate::collaboration) const PROJECT_KIND_GALGAME: &str = "galgame";
pub(in crate::collaboration) const SOURCE_MODE_SELF: &str = "self";
pub(in crate::collaboration) const SOURCE_MODE_FREEFORM: &str = "freeform";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAccessScope {
    All,
    Owned,
    Collaborating,
}

impl ProjectAccessScope {
    pub fn from_optional(value: Option<&str>) -> Result<Self, CollaborationDbError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("all") => Ok(Self::All),
            Some("owned") => Ok(Self::Owned),
            Some("collaborating") => Ok(Self::Collaborating),
            Some(other) => Err(CollaborationDbError::InvalidArgument(format!(
                "项目访问范围无效: {other}"
            ))),
        }
    }
}
