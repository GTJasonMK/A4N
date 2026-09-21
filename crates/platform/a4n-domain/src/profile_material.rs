use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};

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

pub fn profile_material_identity(
    image_type: &str,
    metadata: Option<&Value>,
) -> Option<(&'static str, i64)> {
    let expected_kind = match image_type {
        IMAGE_TYPE_SCENE_MATERIAL => ProfileMaterialKind::Scene,
        IMAGE_TYPE_OBJECT_MATERIAL => ProfileMaterialKind::Object,
        _ => return None,
    };
    let metadata_kind = metadata
        .and_then(|metadata| metadata.get("profile_kind"))
        .and_then(Value::as_str);
    if metadata_kind != Some(expected_kind.as_str()) {
        return None;
    }
    metadata
        .and_then(|metadata| metadata.get("profile_id"))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .map(|id| (expected_kind.as_str(), id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn identity_requires_matching_kind_and_positive_id() {
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_OBJECT_MATERIAL,
                Some(&json!({ "profile_kind": "object", "profile_id": 8 })),
            ),
            Some(("object", 8))
        );
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_SCENE_MATERIAL,
                Some(&json!({ "profile_kind": "object", "profile_id": 8 })),
            ),
            None
        );
        assert_eq!(
            profile_material_identity(
                IMAGE_TYPE_OBJECT_MATERIAL,
                Some(&json!({ "profile_kind": "object", "profile_id": 0 })),
            ),
            None
        );
    }
}
