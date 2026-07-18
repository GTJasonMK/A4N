use super::super::*;

pub(super) fn validate_media_asset_source_context(
    asset: &media_asset::Model,
    root_project_id: i64,
    source_snapshots: &HashMap<i64, &project_source_snapshot::Model>,
    source_snapshot_chapters: &HashMap<i64, &source_snapshot_chapter::Model>,
) -> Result<(), String> {
    let source_snapshot_id = asset.source_snapshot_id.ok_or_else(|| {
        format!(
            "项目族导入包媒体资产缺少 source_snapshot_id: media_asset_id={}",
            asset.id
        )
    })?;
    let snapshot = source_snapshots.get(&source_snapshot_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体资产引用了不存在的源文本快照: media_asset_id={}, source_snapshot_id={source_snapshot_id}",
            asset.id
        )
    })?;
    if snapshot.root_project_id != root_project_id {
        return Err(format!(
            "项目族导入包媒体资产 source_snapshot_id 不属于根项目: media_asset_id={}, source_snapshot_id={source_snapshot_id}",
            asset.id
        ));
    }
    if asset.source_chapter_id.is_some() != asset.source_snapshot_chapter_id.is_some() {
        return Err(format!(
            "项目族导入包媒体资产 source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空: media_asset_id={}",
            asset.id
        ));
    }
    if let Some(source_snapshot_chapter_id) = asset.source_snapshot_chapter_id {
        let snapshot_chapter = source_snapshot_chapters
            .get(&source_snapshot_chapter_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包媒体资产引用了不存在的源快照章节: media_asset_id={}, source_snapshot_chapter_id={source_snapshot_chapter_id}",
                    asset.id
                )
            })?;
        if snapshot_chapter.snapshot_id != source_snapshot_id {
            return Err(format!(
                "项目族导入包媒体资产源快照章节不属于目标快照: media_asset_id={}, source_snapshot_id={}, source_snapshot_chapter_id={}",
                asset.id, source_snapshot_id, source_snapshot_chapter_id
            ));
        }
        let source_chapter_id = asset
            .source_chapter_id
            .expect("media asset chapter context is checked as paired");
        if snapshot_chapter.source_chapter_id != source_chapter_id {
            return Err(format!(
                "项目族导入包媒体资产 source_chapter_id 与 source_snapshot_chapter_id 不一致: media_asset_id={}, source_chapter_id={}, source_snapshot_chapter_id={}, expected_source_chapter_id={}",
                asset.id,
                source_chapter_id,
                source_snapshot_chapter_id,
                snapshot_chapter.source_chapter_id
            ));
        }
    }
    Ok(())
}
