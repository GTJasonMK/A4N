use std::collections::HashMap;

use a4n_core::models::project::source_snapshot_profile;
use serde_json::Value;

use super::super::json_ids::{json_object, remap_object_id, remap_required_object_id};
use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, hash_json, remap_profile_id,
};

pub fn snapshot_profile_key(kind: &str, id: i64) -> String {
    let snapshot_kind = match kind {
        "character_profile" => "character",
        value => value,
    };
    format!("{snapshot_kind}/{id}")
}

pub(in crate::project_family_import) fn remap_profile_hashes_json(
    value: &Value,
    snapshot_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
    maps: &ImportIdMaps,
) -> Result<Value, ProjectFamilyImportError> {
    let rows = value.as_array().ok_or_else(|| {
        ProjectFamilyImportError::invalid_package("源文本快照 profile_hashes_json 必须是数组")
    })?;
    let mut remapped = Vec::with_capacity(rows.len());
    for row in rows {
        let mut object = row.as_object().cloned().ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("源文本快照 profile_hashes_json 项必须是对象")
        })?;
        let kind = object.get("kind").and_then(Value::as_str).ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("profile_hashes_json.kind 必须是字符串")
        })?;
        let old_id = object.get("id").and_then(Value::as_i64).ok_or_else(|| {
            ProjectFamilyImportError::invalid_package("profile_hashes_json.id 必须是整数")
        })?;
        let detail_key = (snapshot_id, snapshot_profile_key(kind, old_id));
        let detail = snapshot_profiles.get(&detail_key).ok_or_else(|| {
            ProjectFamilyImportError::invalid_package(format!(
                "profile_hashes_json 缺少对应源快照档案明细: snapshot_id={snapshot_id}, kind={kind}, id={old_id}"
            ))
        })?;
        let (_, payload_hash) = remap_source_snapshot_profile_payload(detail, maps)?;
        object.insert(
            "id".to_string(),
            Value::from(remap_profile_id(
                kind,
                old_id,
                maps,
                "profile_hashes_json.id",
            )?),
        );
        object.insert("hash".to_string(), Value::from(payload_hash));
        remapped.push(Value::Object(object));
    }
    Ok(Value::Array(remapped))
}

pub(in crate::project_family_import) fn remap_source_snapshot_profile_payload(
    profile: &source_snapshot_profile::Model,
    maps: &ImportIdMaps,
) -> Result<(Value, String), ProjectFamilyImportError> {
    let mut object = json_object(&profile.payload_json, "源快照档案 payload_json")?;
    match profile.profile_kind.as_str() {
        "character" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.character_profile_ids,
                "源快照档案 payload_json",
            )?;
        }
        "object" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.object_profile_ids,
                "源快照档案 payload_json",
            )?;
        }
        "scene" => {
            remap_required_object_id(
                &mut object,
                "id",
                &maps.scene_profile_ids,
                "源快照档案 payload_json",
            )?;
            remap_object_id(
                &mut object,
                "parent_scene_id",
                &maps.scene_profile_ids,
                "源快照档案 payload_json",
            )?;
        }
        other => {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "源快照档案类型无效: {other}"
            )));
        }
    }
    remap_required_object_id(
        &mut object,
        "project_id",
        &maps.project_ids,
        "源快照档案 payload_json",
    )?;
    let payload = Value::Object(object);
    let payload_hash = hash_json(&payload);
    Ok((payload, payload_hash))
}
