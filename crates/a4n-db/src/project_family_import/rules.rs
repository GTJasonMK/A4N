use std::collections::HashMap;

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{ImportIdMaps, ProjectFamilyImportError};
use crate::text_anchor::{
    EVIDENCE_ROLE_APPEARANCE, EVIDENCE_ROLE_BEHAVIOR, EVIDENCE_ROLE_FIRST_SEEN,
    EVIDENCE_ROLE_LOCATION, EVIDENCE_ROLE_RELATIONSHIP, EVIDENCE_ROLE_STATE_CHANGE,
};

const MEDIA_STATUS_COMPLETED: &str = "completed";
const MEDIA_STATUS_PENDING: &str = "pending";

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(in crate::project_family_import) fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    sha256_bytes(&bytes)
}

pub fn normalize_profile_evidence_role(
    role: &str,
) -> Result<&'static str, ProjectFamilyImportError> {
    match role.trim() {
        EVIDENCE_ROLE_FIRST_SEEN => Ok(EVIDENCE_ROLE_FIRST_SEEN),
        EVIDENCE_ROLE_APPEARANCE => Ok(EVIDENCE_ROLE_APPEARANCE),
        EVIDENCE_ROLE_STATE_CHANGE => Ok(EVIDENCE_ROLE_STATE_CHANGE),
        EVIDENCE_ROLE_LOCATION => Ok(EVIDENCE_ROLE_LOCATION),
        EVIDENCE_ROLE_RELATIONSHIP => Ok(EVIDENCE_ROLE_RELATIONSHIP),
        EVIDENCE_ROLE_BEHAVIOR => Ok(EVIDENCE_ROLE_BEHAVIOR),
        "" => Err(ProjectFamilyImportError::InvalidPackage(
            "项目族导入包档案证据 evidence_role 不能为空".to_string(),
        )),
        other => Err(ProjectFamilyImportError::InvalidPackage(format!(
            "项目族导入包档案证据 evidence_role 无效: {other}"
        ))),
    }
}

pub fn media_asset_status_for_import_insert(status: &str) -> &str {
    if status == MEDIA_STATUS_COMPLETED {
        MEDIA_STATUS_PENDING
    } else {
        status
    }
}

pub(in crate::project_family_import) fn map_required(
    map: &HashMap<i64, i64>,
    old_id: i64,
    label: &str,
) -> Result<i64, ProjectFamilyImportError> {
    map.get(&old_id).copied().ok_or_else(|| {
        ProjectFamilyImportError::InvalidPackage(format!(
            "{label} 引用了导入包外或尚未导入的 ID: {old_id}"
        ))
    })
}

pub(in crate::project_family_import) fn map_optional(
    map: &HashMap<i64, i64>,
    old_id: Option<i64>,
    label: &str,
) -> Result<Option<i64>, ProjectFamilyImportError> {
    old_id.map(|id| map_required(map, id, label)).transpose()
}

pub(in crate::project_family_import) fn remap_profile_id(
    profile_kind: &str,
    old_id: i64,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<i64, ProjectFamilyImportError> {
    match profile_kind {
        "character" | "character_profile" => {
            map_required(&maps.character_profile_ids, old_id, label)
        }
        "object" => map_required(&maps.object_profile_ids, old_id, label),
        "scene" => map_required(&maps.scene_profile_ids, old_id, label),
        "event" => Err(ProjectFamilyImportError::InvalidPackage(format!(
            "{label} 引用了尚未持久化建模的 event 实体: {old_id}"
        ))),
        other => Err(ProjectFamilyImportError::InvalidPackage(format!(
            "{label} 实体类型无效: {other}"
        ))),
    }
}

pub(in crate::project_family_import) fn remap_optional_entity_id(
    entity_type: &str,
    old_id: Option<i64>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<Option<i64>, ProjectFamilyImportError> {
    old_id
        .map(|id| remap_profile_id(entity_type, id, maps, label))
        .transpose()
}
