use std::collections::HashMap;

use a4n_domain::shooting_script::{ChapterShootingScriptV1, ShootingVisualPlanSourceV1};
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

pub(in crate::project_family_import) fn media_asset_status_for_import_insert(status: &str) -> &str {
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

pub(in crate::project_family_import) fn remap_chapter_source_revision_key(
    label: &str,
    old_source: (i64, Option<i64>, Option<i64>),
    new_source: (i64, Option<i64>, Option<i64>),
    source_content_sha256: &str,
    source_revision_key: &str,
) -> Result<String, ProjectFamilyImportError> {
    let expected = chapter_source_revision_key(label, old_source, source_content_sha256)?;
    if source_revision_key != expected {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 的 source_revision_key 与来源身份不一致"
        )));
    }
    chapter_source_revision_key(label, new_source, source_content_sha256)
}

fn chapter_source_revision_key(
    label: &str,
    (source_chapter_id, source_snapshot_id, source_snapshot_chapter_id): (
        i64,
        Option<i64>,
        Option<i64>,
    ),
    source_content_sha256: &str,
) -> Result<String, ProjectFamilyImportError> {
    match (source_snapshot_id, source_snapshot_chapter_id) {
        (None, None) => Ok(format!("live:{source_chapter_id}:{source_content_sha256}")),
        (Some(snapshot_id), Some(snapshot_chapter_id)) => Ok(format!(
            "snapshot:{snapshot_id}:chapter:{snapshot_chapter_id}:{source_content_sha256}"
        )),
        _ => Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 的 source_snapshot_id 与 source_snapshot_chapter_id 必须同时存在或同时为空"
        ))),
    }
}

pub(in crate::project_family_import) fn remap_visual_plan_identity(
    visual_plan_id: Option<i64>,
    visual_plan_version: Option<i32>,
    visual_plan_sha256: Option<&str>,
    maps: &ImportIdMaps,
    label: &str,
) -> Result<(Option<i64>, Option<i32>, Option<String>), ProjectFamilyImportError> {
    match (visual_plan_id, visual_plan_version, visual_plan_sha256) {
        (None, None, None) => Ok((None, None, None)),
        (Some(id), Some(version), Some(sha256))
            if version > 0
                && sha256.len() == 64
                && sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok((
                Some(map_required(&maps.visual_plan_ids, id, label)?),
                Some(version),
                Some(sha256.to_string()),
            ))
        }
        _ => Err(ProjectFamilyImportError::invalid_package(format!(
            "{label} 的 visual_plan_id、visual_plan_version、visual_plan_sha256 必须同时有效或同时为空"
        ))),
    }
}

pub(in crate::project_family_import) fn remap_shooting_script_visual_plan_source(
    script_json: &Value,
    old_identity: (Option<i64>, Option<i32>, Option<&str>),
    new_identity: (Option<i64>, Option<i32>, Option<&str>),
    label: &str,
) -> Result<(Value, String), ProjectFamilyImportError> {
    let mut script: ChapterShootingScriptV1 =
        serde_json::from_value(script_json.clone()).map_err(|error| {
            ProjectFamilyImportError::invalid_package(format!("{label} JSON 无效: {error}"))
        })?;
    match (
        old_identity,
        new_identity,
        script.visual_plan_source.as_ref(),
    ) {
        ((None, None, None), (None, None, None), None) => {}
        (
            (Some(old_id), Some(old_version), Some(old_sha256)),
            (Some(new_id), Some(new_version), Some(new_sha256)),
            Some(source),
        ) if source.plan_id == old_id
            && source.plan_version == old_version
            && source.plan_sha256 == old_sha256 =>
        {
            script.visual_plan_source = Some(ShootingVisualPlanSourceV1 {
                plan_id: new_id,
                plan_version: new_version,
                plan_sha256: new_sha256.to_string(),
            });
        }
        _ => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "{label} JSON 与 visual plan 身份不一致"
            )));
        }
    }
    script.validate().map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("{label} 内容无效: {error}"))
    })?;
    let bytes = serde_json::to_vec(&script).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("序列化 {label} 失败: {error}"))
    })?;
    let script_json = serde_json::to_value(&script).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!("序列化 {label} 失败: {error}"))
    })?;
    Ok((script_json, sha256_bytes(&bytes)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_media_asset_import_uses_pending_insert_status() {
        assert_eq!(media_asset_status_for_import_insert("completed"), "pending");
        assert_eq!(media_asset_status_for_import_insert("running"), "running");
        assert_eq!(media_asset_status_for_import_insert("failed"), "failed");
    }

    #[test]
    fn remaps_chapter_source_and_visual_plan_identity_together() {
        let key = remap_chapter_source_revision_key(
            "章节视觉编排 #7",
            (3, Some(4), Some(5)),
            (13, Some(14), Some(15)),
            "hash",
            "snapshot:4:chapter:5:hash",
        )
        .expect("source identity should remap");
        assert_eq!(key, "snapshot:14:chapter:15:hash");

        let mut maps = ImportIdMaps::default();
        maps.visual_plan_ids.insert(7, 17);
        assert_eq!(
            remap_visual_plan_identity(Some(7), Some(2), Some(&"a".repeat(64)), &maps, "方案")
                .expect("visual identity should remap"),
            (Some(17), Some(2), Some("a".repeat(64)))
        );
    }
}
