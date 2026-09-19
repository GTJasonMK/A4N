use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMaterialKind {
    Scene,
    Object,
}

impl ProfileMaterialKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scene => "scene",
            Self::Object => "object",
        }
    }
}

pub fn resolve_downstream_image_ids(
    image_ids: Vec<i64>,
    strategy: crate::DownstreamConflictStrategy,
    subject: &str,
    operation: &str,
) -> Result<Vec<i64>, String> {
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    match strategy {
        crate::DownstreamConflictStrategy::Reject => Err(format!(
            "{subject}已派生 {} 张素材图，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源档案",
            image_ids.len()
        )),
        crate::DownstreamConflictStrategy::KeepDownstream => Ok(Vec::new()),
        crate::DownstreamConflictStrategy::DeleteDownstream => Ok(image_ids),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMaterialKindFilter {
    All,
    Scene,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMaterialState {
    Missing,
    Ready,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMaterialImageStaleness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMaterialFilter {
    All,
    Missing,
    Ready,
    Stale,
    Unknown,
    Favorite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMaterialSort {
    MissingFirst,
    StaleFirst,
    Newest,
    Importance,
}
