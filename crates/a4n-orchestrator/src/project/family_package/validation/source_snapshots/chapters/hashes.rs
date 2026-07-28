use a4n_core::models::project::{project_source_snapshot, source_snapshot_chapter};
use std::collections::{HashMap, HashSet};

use crate::project::{
    required_json_i32_field, required_json_i64_field, required_json_string_field,
    required_optional_json_string_field,
};

use super::super::manifest::SourceSnapshotManifestInfo;

pub(super) fn validate_snapshot_chapter_hashes(
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
