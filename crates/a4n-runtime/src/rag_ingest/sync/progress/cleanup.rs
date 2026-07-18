use super::reporter::{RagIngestProgress, progress_stage};
use super::stats::{ExpectedSyncStats, checked_rag_count_add};
use crate::state::AppState;
use a4n_orchestrator::rag::RagSyncContext;

pub(in crate::rag_ingest) async fn cleanup_stale_records(
    sync: &mut RagSyncContext,
    state: &AppState,
    stats: &mut ExpectedSyncStats,
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    label: &str,
) -> Result<(), String> {
    let deleted = sync
        .cleanup_stale(&state.orchestrator)
        .await
        .map_err(|error| error.to_string())?;
    if deleted > 0 {
        stats.deleted_count = checked_rag_count_add(stats.deleted_count, deleted, "删除记录数")?;
        progress_stage(progress, format!("已清理过期{label} RAG 记录 {deleted} 个"))?;
    }
    Ok(())
}
