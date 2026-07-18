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
) -> Result<serde_json::Value, String> {
    let trace_id =
        crate::sidecar_clients::scene_rag_sync_operation_trace_id(project_id, trace_source)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        sync_scene_rag_records_inner(state, project_id),
    )
    .await
}

async fn sync_scene_rag_records_inner(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let mut total_stats = IngestStats::default();

    let scene_profile_stats = ingest_scene_profiles_inner(state, project_id, None).await?;
    let scene_profile_count = scene_profile_stats.item_count;
    total_stats.merge(scene_profile_stats)?;

    let scene_occurrence_stats = ingest_scene_occurrences_inner(state, project_id, None).await?;
    let scene_occurrence_count = scene_occurrence_stats.item_count;
    total_stats.merge(scene_occurrence_stats)?;

    let scene_transition_stats = ingest_scene_transitions_inner(state, project_id, None).await?;
    let scene_transition_count = scene_transition_stats.item_count;
    total_stats.merge(scene_transition_stats)?;

    if !total_stats.errors.is_empty() {
        return Err(scene_rag_sync_error_message(&total_stats.errors));
    }

    Ok(serde_json::json!({
        "scene_profile_count": scene_profile_count,
        "scene_occurrence_count": scene_occurrence_count,
        "scene_transition_count": scene_transition_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
    }))
}
