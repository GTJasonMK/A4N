use a4n_domain::project::ProjectSourceMode;

use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary,
    types::{MediaSourceLineageContext, ProjectLineageContext},
};

pub(in crate::asset_lineage_sync) fn media_source_context_for_row(
    project_context: &ProjectLineageContext,
    source_chapter_id: Option<i64>,
    table: &str,
    row_id: i64,
    summary: &mut AssetLineageBackfillSummary,
) -> Option<MediaSourceLineageContext> {
    if project_context.source_mode == ProjectSourceMode::Freeform {
        if source_chapter_id.is_some() {
            summary.warnings.push(format!(
                "{table} #{row_id} 属于 freeform 项目但绑定了章节 {:?}，无法写入 media_asset，已跳过。",
                source_chapter_id
            ));
            return None;
        }
        return Some(MediaSourceLineageContext {
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
        });
    }

    let Some(snapshot) = project_context.snapshot.as_ref() else {
        summary.warnings.push(format!(
            "{table} #{row_id} 所属项目族缺少源文本快照，无法写入 media_asset，已跳过。"
        ));
        return None;
    };

    let Some(chapter_id) = source_chapter_id else {
        return Some(MediaSourceLineageContext {
            source_snapshot_id: Some(snapshot.id),
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
        });
    };
    let Some(snapshot_chapter) = project_context.snapshot_chapters_by_source.get(&chapter_id)
    else {
        summary.warnings.push(format!(
            "{table} #{row_id} 的章节 {chapter_id} 不在源文本快照 {} 中，无法写入 media_asset，已跳过。",
            snapshot.id
        ));
        return None;
    };
    Some(MediaSourceLineageContext {
        source_snapshot_id: Some(snapshot.id),
        source_chapter_id: Some(chapter_id),
        source_snapshot_chapter_id: Some(snapshot_chapter.id),
    })
}
