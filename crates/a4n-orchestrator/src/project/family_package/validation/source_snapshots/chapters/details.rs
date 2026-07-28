use a4n_core::models::project::source_snapshot_chapter;
use std::collections::{HashMap, HashSet};

use super::super::super::super::{ProjectFamilyPackage, hash_json, hash_text};
use super::super::manifest::SourceSnapshotManifestInfo;
use super::hashes::validate_snapshot_chapter_hashes;

pub(in super::super) fn validate_source_snapshot_chapter_manifest(
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
