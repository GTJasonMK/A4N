use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary,
    types::{FamilyLineageContext, SourceLineageContext},
};

pub(in crate::asset_lineage_sync) fn source_context_for_row(
    family: &FamilyLineageContext,
    project_id: i64,
    source_chapter_id: Option<i64>,
    table: &str,
    row_id: i64,
    summary: &mut AssetLineageBackfillSummary,
) -> Option<SourceLineageContext> {
    let Some(project_context) = family.contexts_by_project_id.get(&project_id) else {
        summary.warnings.push(format!(
            "{table} #{row_id} 所属项目 {project_id} 不在当前项目族中，已跳过。"
        ));
        return None;
    };
    let Some(snapshot) = project_context.snapshot.as_ref() else {
        summary.warnings.push(format!(
            "{table} #{row_id} 所属项目族缺少源文本快照，无法写入 generation_spec，已跳过。"
        ));
        return None;
    };
    let source_snapshot_chapter_id = source_chapter_id.and_then(|chapter_id| {
        project_context
            .snapshot_chapters_by_source
            .get(&chapter_id)
            .map(|chapter| chapter.id)
    });
    if source_chapter_id.is_some() && source_snapshot_chapter_id.is_none() {
        summary.warnings.push(format!(
            "{table} #{row_id} 的章节 {:?} 不在源文本快照 {} 中，无法写入 generation_spec，已跳过。",
            source_chapter_id, snapshot.id
        ));
        return None;
    }
    Some(SourceLineageContext {
        project_id: project_context.project_id,
        root_project_id: project_context.root_project_id,
        source_snapshot_id: snapshot.id,
        source_version_hash: snapshot.source_version_hash.clone(),
        source_chapter_id,
        source_snapshot_chapter_id,
    })
}
