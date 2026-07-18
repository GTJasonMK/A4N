use std::collections::{HashMap, HashSet};

use a4n_core::models::project::source_snapshot_profile;

use crate::actions::project_transfer::validation::{
    required_json_i64_field, required_json_string_field,
};

use super::super::super::{ProjectFamilyPackage, snapshot_profile_key};
use super::super::manifest::SourceSnapshotManifestInfo;
use super::super::rules::{is_valid_snapshot_profile_kind, validate_snapshot_payload_identity};

pub(in super::super) fn validate_source_snapshot_profile_manifest(
    package: &ProjectFamilyPackage,
    snapshots: &HashMap<i64, SourceSnapshotManifestInfo>,
) -> Result<(), String> {
    let mut profiles_by_snapshot: HashMap<i64, HashMap<String, &source_snapshot_profile::Model>> =
        HashMap::new();
    let mut seen_profiles = HashSet::new();
    for profile in &package.source_snapshot_profiles {
        validate_snapshot_payload_identity(
            snapshots,
            profile.snapshot_id,
            profile.root_project_id,
            profile.id,
            &profile.profile_kind,
            profile.source_profile_id,
            &profile.payload_json,
            &profile.payload_hash,
            "源快照档案",
            "source_snapshot_profile_id",
            "profile_kind",
            "source_profile_id",
            is_valid_snapshot_profile_kind,
        )?;
        let key = snapshot_profile_key(&profile.profile_kind, profile.source_profile_id);
        if !seen_profiles.insert((profile.snapshot_id, key.clone())) {
            return Err(format!(
                "项目族导入包源快照档案重复: snapshot_id={}, profile_kind={}, source_profile_id={}",
                profile.snapshot_id, profile.profile_kind, profile.source_profile_id
            ));
        }
        profiles_by_snapshot
            .entry(profile.snapshot_id)
            .or_default()
            .insert(key, profile);
    }

    for snapshot in &package.source_snapshots {
        let rows = snapshot.profile_hashes_json.as_array().ok_or_else(|| {
            format!(
                "项目族导入包源文本快照 profile_hashes_json 必须是数组: source_snapshot_id={}",
                snapshot.id
            )
        })?;
        let profiles = profiles_by_snapshot.get(&snapshot.id);
        let mut seen_hash_rows = HashSet::new();
        for row in rows {
            let object = row.as_object().ok_or_else(|| {
                format!(
                    "项目族导入包源文本快照 profile_hashes_json 项必须是对象: source_snapshot_id={}",
                    snapshot.id
                )
            })?;
            let kind = required_json_string_field(object, "kind", "profile_hashes_json")?;
            if !is_valid_snapshot_profile_kind(kind) {
                return Err(format!(
                    "项目族导入包源文本快照 profile_hashes_json 档案类型无效: source_snapshot_id={}, kind={}",
                    snapshot.id, kind
                ));
            }
            let id = required_json_i64_field(object, "id", "profile_hashes_json")?;
            let hash = required_json_string_field(object, "hash", "profile_hashes_json")?;
            let key = snapshot_profile_key(kind, id);
            if !seen_hash_rows.insert(key.clone()) {
                return Err(format!(
                    "项目族导入包源文本快照 profile_hashes_json 档案重复: source_snapshot_id={}, kind={}, id={}",
                    snapshot.id, kind, id
                ));
            }
            let detail = profiles.and_then(|profiles| profiles.get(&key)).ok_or_else(|| {
                format!(
                    "项目族导入包源文本快照 profile_hashes_json 缺少对应档案明细: source_snapshot_id={}, kind={}, id={}",
                    snapshot.id, kind, id
                )
            })?;
            if detail.payload_hash != hash {
                return Err(format!(
                    "项目族导入包源文本快照 profile_hashes_json 与档案明细 hash 不一致: source_snapshot_id={}, kind={}, id={}",
                    snapshot.id, kind, id
                ));
            }
        }
        if let Some(profiles) = profiles {
            for key in profiles.keys() {
                if !seen_hash_rows.contains(key) {
                    return Err(format!(
                        "项目族导入包源文本快照 profile_hashes_json 缺少档案: source_snapshot_id={}, profile={}",
                        snapshot.id, key
                    ));
                }
            }
        }
    }
    Ok(())
}
