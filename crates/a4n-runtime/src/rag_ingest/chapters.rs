use super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, RagSyncContext, cleanup_stale_records,
    labeled_rag_record, progress_stage, sync_labeled_records_concurrent,
};
use crate::state::AppState;
use a4n_core::models::chapter::chapter;
use sea_orm::*;

pub(crate) async fn ingest_chapters_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let mut sync =
        RagSyncContext::for_id_prefixes(state, project_id, vec![format!("chapter:{project_id}:")])
            .await?;
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    for (index, chapter) in chapters.iter().enumerate() {
        if !chapter_should_sync_rag_records(chapter) {
            continue;
        }
        progress_stage(
            &mut progress,
            format!(
                "正在同步章节正文 {}/{}：第{}章",
                index + 1,
                chapters.len(),
                chapter.chapter_number
            ),
        )?;
        let records =
            crate::workflow::chapter_rag_records(chapter, state.embedding.as_ref()).await?;
        let labeled_records = records
            .into_iter()
            .map(|record| {
                let label = crate::workflow::chapter_rag_record_label(chapter, &record);
                let chunk_id = record.chunk_id.clone();
                labeled_rag_record(
                    record,
                    label,
                    format!("章节 RAG：chapter_id={}, chunk_id={chunk_id}", chapter.id),
                )
            })
            .collect::<Vec<_>>();
        let processed = sync_labeled_records_concurrent(
            &mut sync,
            state,
            labeled_records,
            &mut stats,
            &mut progress,
        )
        .await?;
        if processed > 0 {
            stats.item_count += 1;
        } else {
            stats
                .errors
                .push(format!("章节 {} 未写入任何 RAG 分块", chapter.id));
        }
    }

    cleanup_stale_records(&mut sync, state, &mut stats, &mut progress, "章节派生").await?;
    Ok(stats)
}

pub(super) fn chapter_should_sync_rag_records(chapter: &chapter::Model) -> bool {
    chapter.has_available_content()
}
