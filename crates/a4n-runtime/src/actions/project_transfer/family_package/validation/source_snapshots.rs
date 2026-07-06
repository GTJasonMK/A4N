use std::collections::{HashMap, HashSet};

use a4n_core::models::project::{
    project_source_snapshot, source_snapshot_chapter, source_snapshot_profile,
};
use serde_json::Value;

use crate::actions::project_transfer::validation::{
    required_json_i32_field, required_json_i64_field, required_json_string_field,
    required_optional_json_string_field,
};

use super::super::{ProjectFamilyPackage, hash_json, hash_text};
use super::snapshot_profile_key;

pub(super) fn validate_source_snapshot_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let mut snapshots = HashMap::new();
    for snapshot in &package.source_snapshots {
        if snapshot.root_project_id != package.root_project_id {
            return Err(format!(
                "项目族导入包源文本快照不属于根项目: source_snapshot_id={}, root_project_id={}",
                snapshot.id, snapshot.root_project_id
            ));
        }
        let (start_chapter, end_chapter) = parse_chapter_scope_json(
            &snapshot.chapter_scope_json,
            "源文本快照 chapter_scope_json",
        )?;
        snapshots.insert(
            snapshot.id,
            SourceSnapshotManifestInfo {
                root_project_id: snapshot.root_project_id,
                start_chapter,
                end_chapter,
            },
        );
    }
    validate_source_snapshot_chapter_manifest(package, &snapshots)?;
    validate_source_snapshot_outline_manifest(package, &snapshots)?;
    validate_source_snapshot_profile_manifest(package, &snapshots)?;
    Ok(())
}

#[derive(Clone, Copy)]
struct SourceSnapshotManifestInfo {
    root_project_id: i64,
    start_chapter: i32,
    end_chapter: i32,
}

pub(super) fn parse_chapter_scope_json(value: &Value, label: &str) -> Result<(i32, i32), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} 必须是 JSON 对象"))?;
    let start = object
        .get("start")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.start 必须是整数"))?;
    let end = object
        .get("end")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("{label}.end 必须是整数"))?;
    if start <= 0 || end < start || start > i32::MAX as i64 || end > i32::MAX as i64 {
        return Err(format!("{label} 范围无效: start={start}, end={end}"));
    }
    Ok((start as i32, end as i32))
}

fn validate_source_snapshot_chapter_manifest(
    package: &ProjectFamilyPackage,
    snapshots: &HashMap<i64, SourceSnapshotManifestInfo>,
) -> Result<(), String> {
    let mut chapters_by_snapshot: HashMap<i64, Vec<&source_snapshot_chapter::Model>> =
        HashMap::new();
    let mut seen_numbers = HashSet::new();
    let mut seen_source_ids = HashSet::new();
    for chapter in &package.source_snapshot_chapters {
        let snapshot = snapshots.get(&chapter.snapshot_id).ok_or_else(|| {
            format!(
                "项目族导入包源快照章节引用了不存在的源文本快照: source_snapshot_chapter_id={}, snapshot_id={}",
                chapter.id, chapter.snapshot_id
            )
        })?;
        if chapter.root_project_id != snapshot.root_project_id {
            return Err(format!(
                "项目族导入包源快照章节根项目不一致: source_snapshot_chapter_id={}, root_project_id={}",
                chapter.id, chapter.root_project_id
            ));
        }
        if chapter.chapter_number < snapshot.start_chapter
            || chapter.chapter_number > snapshot.end_chapter
        {
            return Err(format!(
                "项目族导入包源快照章节不在快照范围内: source_snapshot_chapter_id={}, chapter_number={}, range={}-{}",
                chapter.id, chapter.chapter_number, snapshot.start_chapter, snapshot.end_chapter
            ));
        }
        if !seen_numbers.insert((chapter.snapshot_id, chapter.chapter_number)) {
            return Err(format!(
                "项目族导入包源快照章节号重复: snapshot_id={}, chapter_number={}",
                chapter.snapshot_id, chapter.chapter_number
            ));
        }
        if !seen_source_ids.insert((chapter.snapshot_id, chapter.source_chapter_id)) {
            return Err(format!(
                "项目族导入包源快照章节 source_chapter_id 重复: snapshot_id={}, source_chapter_id={}",
                chapter.snapshot_id, chapter.source_chapter_id
            ));
        }
        let expected_content_hash = hash_text(&chapter.content);
        if chapter.content_hash != expected_content_hash {
            return Err(format!(
                "项目族导入包源快照章节 content_hash 与正文不一致: source_snapshot_chapter_id={}",
                chapter.id
            ));
        }
        let expected_analysis_hash = chapter.analysis_data_json.as_ref().map(hash_json);
        if chapter.analysis_hash != expected_analysis_hash {
            return Err(format!(
                "项目族导入包源快照章节 analysis_hash 与 analysis_data_json 不一致: source_snapshot_chapter_id={}",
                chapter.id
            ));
        }
        chapters_by_snapshot
            .entry(chapter.snapshot_id)
            .or_default()
            .push(chapter);
    }

    for snapshot in &package.source_snapshots {
        let info = snapshots
            .get(&snapshot.id)
            .expect("source snapshot was validated before detail checks");
        let mut chapters_by_number = HashMap::new();
        if let Some(chapters) = chapters_by_snapshot.get(&snapshot.id) {
            for chapter in chapters {
                chapters_by_number.insert(chapter.chapter_number, *chapter);
            }
        }
        for chapter_number in info.start_chapter..=info.end_chapter {
            if !chapters_by_number.contains_key(&chapter_number) {
                return Err(format!(
                    "项目族导入包源文本快照缺少章节明细: source_snapshot_id={}, chapter_number={}",
                    snapshot.id, chapter_number
                ));
            }
        }
        validate_snapshot_chapter_hashes(snapshot, *info, &chapters_by_number)?;
    }
    Ok(())
}

fn validate_snapshot_chapter_hashes(
    snapshot: &project_source_snapshot::Model,
    info: SourceSnapshotManifestInfo,
    chapters_by_number: &HashMap<i32, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    let rows = snapshot.chapter_hashes_json.as_array().ok_or_else(|| {
        format!(
            "项目族导入包源文本快照 chapter_hashes_json 必须是数组: source_snapshot_id={}",
            snapshot.id
        )
    })?;
    let mut seen_numbers = HashSet::new();
    let mut seen_source_ids = HashSet::new();
    for row in rows {
        let object = row.as_object().ok_or_else(|| {
            format!(
                "项目族导入包源文本快照 chapter_hashes_json 项必须是对象: source_snapshot_id={}",
                snapshot.id
            )
        })?;
        let source_chapter_id =
            required_json_i64_field(object, "source_chapter_id", "chapter_hashes_json")?;
        let chapter_number =
            required_json_i32_field(object, "chapter_number", "chapter_hashes_json")?;
        if chapter_number < info.start_chapter || chapter_number > info.end_chapter {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json 章节号不在快照范围内: source_snapshot_id={}, chapter_number={}, range={}-{}",
                snapshot.id, chapter_number, info.start_chapter, info.end_chapter
            ));
        }
        if !seen_numbers.insert(chapter_number) {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json 章节号重复: source_snapshot_id={}, chapter_number={}",
                snapshot.id, chapter_number
            ));
        }
        if !seen_source_ids.insert(source_chapter_id) {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json source_chapter_id 重复: source_snapshot_id={}, source_chapter_id={}",
                snapshot.id, source_chapter_id
            ));
        }
        let content_hash =
            required_json_string_field(object, "content_hash", "chapter_hashes_json")?;
        let analysis_hash =
            required_optional_json_string_field(object, "analysis_hash", "chapter_hashes_json")?;
        let detail = chapters_by_number.get(&chapter_number).ok_or_else(|| {
            format!(
                "项目族导入包源文本快照 chapter_hashes_json 缺少对应章节明细: source_snapshot_id={}, chapter_number={}",
                snapshot.id, chapter_number
            )
        })?;
        if detail.source_chapter_id != source_chapter_id {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json 与章节明细 source_chapter_id 不一致: source_snapshot_id={}, chapter_number={}",
                snapshot.id, chapter_number
            ));
        }
        if detail.content_hash != content_hash {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json 与章节明细 content_hash 不一致: source_snapshot_id={}, chapter_number={}",
                snapshot.id, chapter_number
            ));
        }
        if detail.analysis_hash.as_deref() != analysis_hash {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json 与章节明细 analysis_hash 不一致: source_snapshot_id={}, chapter_number={}",
                snapshot.id, chapter_number
            ));
        }
    }
    for chapter_number in info.start_chapter..=info.end_chapter {
        if !seen_numbers.contains(&chapter_number) {
            return Err(format!(
                "项目族导入包源文本快照 chapter_hashes_json 缺少章节: source_snapshot_id={}, chapter_number={}",
                snapshot.id, chapter_number
            ));
        }
    }
    Ok(())
}

fn validate_source_snapshot_outline_manifest(
    package: &ProjectFamilyPackage,
    snapshots: &HashMap<i64, SourceSnapshotManifestInfo>,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for outline in &package.source_snapshot_outlines {
        validate_snapshot_payload_identity(
            snapshots,
            outline.snapshot_id,
            outline.root_project_id,
            outline.id,
            &outline.outline_kind,
            outline.source_outline_id,
            &outline.payload_json,
            &outline.payload_hash,
            "源快照大纲",
            "source_snapshot_outline_id",
            "outline_kind",
            "source_outline_id",
            is_valid_snapshot_outline_kind,
        )?;
        if !seen.insert((
            outline.snapshot_id,
            outline.outline_kind.clone(),
            outline.source_outline_id,
        )) {
            return Err(format!(
                "项目族导入包源快照大纲重复: snapshot_id={}, outline_kind={}, source_outline_id={}",
                outline.snapshot_id, outline.outline_kind, outline.source_outline_id
            ));
        }
    }
    Ok(())
}

fn validate_source_snapshot_profile_manifest(
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

fn validate_snapshot_payload_identity(
    snapshots: &HashMap<i64, SourceSnapshotManifestInfo>,
    snapshot_id: i64,
    root_project_id: i64,
    row_id: i64,
    kind: &str,
    source_id: i64,
    payload_json: &Value,
    payload_hash: &str,
    row_label: &str,
    row_id_field: &str,
    kind_field: &str,
    source_id_field: &str,
    is_valid_kind: fn(&str) -> bool,
) -> Result<(), String> {
    let snapshot = snapshots.get(&snapshot_id).ok_or_else(|| {
        format!(
            "项目族导入包{row_label}引用了不存在的源文本快照: {row_id_field}={row_id}, snapshot_id={snapshot_id}"
        )
    })?;
    if root_project_id != snapshot.root_project_id {
        return Err(format!(
            "项目族导入包{row_label}根项目不一致: {row_id_field}={row_id}, root_project_id={root_project_id}"
        ));
    }
    if !is_valid_kind(kind) {
        return Err(format!(
            "项目族导入包{row_label}类型无效: {row_id_field}={row_id}, {kind_field}={kind}"
        ));
    }
    let payload = payload_json.as_object().ok_or_else(|| {
        format!("项目族导入包{row_label} payload_json 必须是 JSON 对象: {row_id_field}={row_id}")
    })?;
    let payload_id = required_json_i64_field(payload, "id", &format!("{row_label} payload_json"))?;
    if payload_id != source_id {
        return Err(format!(
            "项目族导入包{row_label} payload_json.id 与 {source_id_field} 不一致: {row_id_field}={row_id}, payload_id={payload_id}, {source_id_field}={source_id}"
        ));
    }
    let payload_project_id =
        required_json_i64_field(payload, "project_id", &format!("{row_label} payload_json"))?;
    if payload_project_id != root_project_id {
        return Err(format!(
            "项目族导入包{row_label} payload_json.project_id 与 root_project_id 不一致: {row_id_field}={row_id}, payload_project_id={payload_project_id}, root_project_id={root_project_id}"
        ));
    }
    let expected_hash = hash_json(payload_json);
    if payload_hash != expected_hash {
        return Err(format!(
            "项目族导入包{row_label} payload_hash 与 payload_json 不一致: {row_id_field}={row_id}"
        ));
    }
    Ok(())
}

fn is_valid_snapshot_outline_kind(kind: &str) -> bool {
    matches!(kind, "part" | "chapter")
}

fn is_valid_snapshot_profile_kind(kind: &str) -> bool {
    matches!(kind, "character" | "object" | "scene")
}
