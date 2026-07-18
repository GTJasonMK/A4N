use std::collections::BTreeSet;

use a4n_core::models::project::text_anchor;

use crate::text_anchor::{TextAnchorDbError, TextAnchorDbResult};

pub(super) fn normalize_anchor_ids(anchor_ids: &[i64]) -> TextAnchorDbResult<BTreeSet<i64>> {
    if anchor_ids.is_empty() {
        return Err(TextAnchorDbError::InvalidArgument(
            "text_anchor_ids 不能为空".to_string(),
        ));
    }
    let mut unique_ids = BTreeSet::new();
    for id in anchor_ids {
        if *id <= 0 {
            return Err(TextAnchorDbError::InvalidArgument(format!(
                "text_anchor_id 必须为正整数: {id}"
            )));
        }
        unique_ids.insert(*id);
    }
    Ok(unique_ids)
}

pub(super) fn validate_loaded_anchor_records(
    anchors: &[text_anchor::Model],
    requested_anchor_count: usize,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_ids: &[i64],
) -> TextAnchorDbResult<()> {
    if anchors.len() != requested_anchor_count {
        return Err(TextAnchorDbError::InvalidArgument(
            "存在不存在的 text_anchor_id".to_string(),
        ));
    }
    let allowed_chapters = source_snapshot_chapter_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for anchor in anchors {
        if anchor.project_id != root_project_id {
            return Err(TextAnchorDbError::InvalidArgument(format!(
                "text_anchor_id={} 不属于文本根项目 {}",
                anchor.id, root_project_id
            )));
        }
        if anchor.source_snapshot_id != source_snapshot_id {
            return Err(TextAnchorDbError::InvalidArgument(format!(
                "text_anchor_id={} 不属于目标源快照",
                anchor.id
            )));
        }
        if !allowed_chapters.is_empty() {
            let chapter_id = anchor.source_snapshot_chapter_id.ok_or_else(|| {
                TextAnchorDbError::InvalidArgument(format!(
                    "text_anchor_id={} 不是源快照章节锚点",
                    anchor.id
                ))
            })?;
            if !allowed_chapters.contains(&chapter_id) {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "text_anchor_id={} 不属于目标章节范围",
                    anchor.id
                )));
            }
        }
    }
    Ok(())
}
