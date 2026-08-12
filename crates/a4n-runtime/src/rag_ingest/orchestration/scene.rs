use super::super::contracts::SceneRagSyncTraceSource;
use super::super::scenes::{
    ingest_scene_occurrences_inner, ingest_scene_profiles_inner, ingest_scene_transitions_inner,
};
use super::super::sync::ExpectedSyncStats as IngestStats;
use super::errors::scene_rag_sync_error_message;
use crate::state::AppState;

pub async fn sync_scene_rag_records(
    state: &AppState,
    project_id: i64,
    trace_source: SceneRagSyncTraceSource,
) -> Result<(), String> {
    let trace_id =
        crate::sidecar_clients::scene_rag_sync_operation_trace_id(project_id, trace_source)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        sync_scene_rag_records_inner(state, project_id),
    )
    .await
}

async fn sync_scene_rag_records_inner(state: &AppState, project_id: i64) -> Result<(), String> {
    let mut total_stats = IngestStats::default();

    total_stats.merge(ingest_scene_profiles_inner(state, project_id, None).await?)?;

    total_stats.merge(ingest_scene_occurrences_inner(state, project_id, None).await?)?;

    total_stats.merge(ingest_scene_transitions_inner(state, project_id, None).await?)?;

    if !total_stats.errors.is_empty() {
        return Err(scene_rag_sync_error_message(&total_stats.errors));
    }

    Ok(())
}
