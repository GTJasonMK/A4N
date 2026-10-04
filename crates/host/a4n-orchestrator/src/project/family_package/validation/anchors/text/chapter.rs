use std::collections::{HashMap, HashSet};

use super::super::super::super::hash_text;

pub(super) fn validate_chapter_content_anchor(
    anchor: &a4n_domain::project::ProjectFamilyTextAnchorRecord,
    chapters: &HashMap<i64, &a4n_domain::project::ProjectFamilyChapterRecord>,
    source_snapshots: &HashSet<i64>,
    snapshot_chapters: &HashMap<
        i64,
        &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
    >,
) -> Result<(), String> {
    validate_chapter_anchor_source(anchor, chapters, source_snapshots, snapshot_chapters)?;
    let expected_source_version_hash = if anchor.source_snapshot_id.is_some() {
        let snapshot_chapter = snapshot_chapters
            .get(&anchor.source_id)
            .expect("snapshot chapter source was validated before hash check");
        snapshot_chapter.content_hash.as_str()
    } else {
        let chapter = chapters
            .get(&anchor.source_id)
            .expect("chapter source was validated before hash check");
        return validate_text_anchor_source_version_hash(anchor, &hash_text(&chapter.content));
    };
    validate_text_anchor_source_version_hash(anchor, expected_source_version_hash)
}

pub(super) fn validate_chapter_anchor_source(
    anchor: &a4n_domain::project::ProjectFamilyTextAnchorRecord,
    chapters: &HashMap<i64, &a4n_domain::project::ProjectFamilyChapterRecord>,
    source_snapshots: &HashSet<i64>,
    snapshot_chapters: &HashMap<
        i64,
        &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord,
    >,
) -> Result<(), String> {
    if let Some(snapshot_id) = anchor.source_snapshot_id {
        if !source_snapshots.contains(&snapshot_id) {
            return Err(format!(
                "项目族导入包文本锚点引用了不存在的源文本快照: text_anchor_id={}, source_snapshot_id={snapshot_id}",
                anchor.id
            ));
        }
        let source_snapshot_chapter_id = anchor.source_snapshot_chapter_id.ok_or_else(|| {
            format!(
                "项目族导入包源快照文本锚点缺少 source_snapshot_chapter_id: text_anchor_id={}",
                anchor.id
            )
        })?;
        if source_snapshot_chapter_id != anchor.source_id {
            return Err(format!(
                "项目族导入包源快照文本锚点 source_id 与 source_snapshot_chapter_id 不一致: text_anchor_id={}, source_id={}, source_snapshot_chapter_id={}",
                anchor.id, anchor.source_id, source_snapshot_chapter_id
            ));
        }
        let snapshot_chapter = snapshot_chapters
            .get(&source_snapshot_chapter_id)
            .ok_or_else(|| {
                format!(
                    "项目族导入包文本锚点引用了不存在的源快照章节: text_anchor_id={}, source_snapshot_chapter_id={source_snapshot_chapter_id}",
                    anchor.id
                )
            })?;
        if snapshot_chapter.snapshot_id != snapshot_id {
            return Err(format!(
                "项目族导入包文本锚点源快照章节不属于目标快照: text_anchor_id={}, source_snapshot_id={}, source_snapshot_chapter_id={}",
                anchor.id, snapshot_id, source_snapshot_chapter_id
            ));
        }
        if anchor.source_chapter_id != Some(snapshot_chapter.source_chapter_id) {
            return Err(format!(
                "项目族导入包文本锚点 source_chapter_id 与源快照章节不一致: text_anchor_id={}, source_chapter_id={:?}, expected={}",
                anchor.id, anchor.source_chapter_id, snapshot_chapter.source_chapter_id
            ));
        }
        return Ok(());
    }

    if anchor.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "项目族导入包非快照文本锚点不能包含 source_snapshot_chapter_id: text_anchor_id={}",
            anchor.id
        ));
    }
    if anchor.source_chapter_id != Some(anchor.source_id) {
        return Err(format!(
            "项目族导入包文本锚点 source_id 与 source_chapter_id 不一致: text_anchor_id={}, source_id={}, source_chapter_id={:?}",
            anchor.id, anchor.source_id, anchor.source_chapter_id
        ));
    }
    if !chapters.contains_key(&anchor.source_id) {
        return Err(format!(
            "项目族导入包文本锚点引用了不存在的章节: text_anchor_id={}, source_id={}",
            anchor.id, anchor.source_id
        ));
    }
    Ok(())
}

fn validate_text_anchor_source_version_hash(
    anchor: &a4n_domain::project::ProjectFamilyTextAnchorRecord,
    expected: &str,
) -> Result<(), String> {
    if anchor.source_version_hash != expected {
        return Err(format!(
            "项目族导入包文本锚点 source_version_hash 与源内容不一致: text_anchor_id={}",
            anchor.id
        ));
    }
    Ok(())
}
