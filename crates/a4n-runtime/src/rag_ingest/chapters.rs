use super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, RagSyncContext, cleanup_stale_records,
    labeled_rag_record, progress_stage, sync_labeled_records_concurrent,
};
use crate::state::AppState;
use a4n_core::models::chapter::chapter;

pub(crate) async fn ingest_chapters_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let mut sync = RagSyncContext::for_id_prefixes(
        &state.orchestrator,
        project_id,
        vec![format!("chapter:{project_id}:")],
    )
    .await
    .map_err(|error| error.to_string())?;
    let chapters = state
        .orchestrator
        .chapters_for_rag_ingest(project_id)
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
        let records = state
            .orchestrator
            .chapter_rag_records(chapter, super::chapter_rag_chunking_mode())
            .await?;
        let labeled_records = records
            .into_iter()
            .map(|record| {
                let label = a4n_orchestrator::rag::chapter_rag_record_label(chapter, &record);
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
