use super::super::chapters::ingest_chapters_inner;
use super::super::characters::{
    ingest_character_attribute_events_inner, ingest_character_records_inner,
    ingest_character_states_inner, ingest_foreshadowing_inner, ingest_protagonists_inner,
    ingest_relationships_inner,
};
use super::super::expected_records::expected_rag_records;
use super::super::scenes::{
    ingest_scene_occurrences_inner, ingest_scene_profiles_inner, ingest_scene_transitions_inner,
};
use super::super::story_records::{
    ingest_blueprint_metadata_inner, ingest_blueprints_inner, ingest_outlines_inner,
    ingest_part_outlines_inner, ingest_world_setting_inner,
};
use super::super::sync::{ExpectedSyncStats as IngestStats, RagIngestProgress};
use super::errors::rag_ingest_error_message;
use crate::{actions::access, state::AppState};

pub async fn rag_ingest_all(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    task_id: Option<&str>,
    rebuild: bool,
    progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<serde_json::Value, String> {
    let trace_id = crate::sidecar_clients::rag_ingest_operation_trace_id(project_id, task_id)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        rag_ingest_all_inner(state, user_id, project_id, rebuild, progress),
    )
    .await
}

async fn rag_ingest_all_inner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    rebuild: bool,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<serde_json::Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    if rebuild {
        state
            .orchestrator()
            .clear_project_rag(project_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    expected_rag_records(state, project_id).await?;
    if let Some(progress) = progress.as_deref_mut() {
        progress.stage("正在检查已同步 RAG 数据")?;
    }
    let mut total_stats = IngestStats::default();

    let chapter_stats = ingest_chapters_inner(state, project_id, progress.as_deref_mut()).await?;
    let chapter_count = chapter_stats.item_count;
    total_stats.merge(chapter_stats)?;

    let foreshadowing_stats =
        ingest_foreshadowing_inner(state, project_id, progress.as_deref_mut()).await?;
    let foreshadowing_count = foreshadowing_stats.item_count;
    total_stats.merge(foreshadowing_stats)?;

    let outline_stats = ingest_outlines_inner(state, project_id, progress.as_deref_mut()).await?;
    let outline_count = outline_stats.item_count;
    total_stats.merge(outline_stats)?;

    let part_stats = ingest_part_outlines_inner(state, project_id, progress.as_deref_mut()).await?;
    let part_outline_count = part_stats.item_count;
    total_stats.merge(part_stats)?;

    let blueprint_stats =
        ingest_blueprints_inner(state, project_id, progress.as_deref_mut()).await?;
    let blueprint_count = blueprint_stats.item_count;
    total_stats.merge(blueprint_stats)?;

    let blueprint_metadata_stats =
        ingest_blueprint_metadata_inner(state, project_id, progress.as_deref_mut()).await?;
    let blueprint_metadata_count = blueprint_metadata_stats.item_count;
    total_stats.merge(blueprint_metadata_stats)?;

    let world_setting_stats =
        ingest_world_setting_inner(state, project_id, progress.as_deref_mut()).await?;
    let world_setting_count = world_setting_stats.item_count;
    total_stats.merge(world_setting_stats)?;

    let character_stats =
        ingest_character_records_inner(state, project_id, progress.as_deref_mut()).await?;
    let character_count = character_stats.item_count;
    total_stats.merge(character_stats)?;

    let protagonist_stats =
        ingest_protagonists_inner(state, project_id, progress.as_deref_mut()).await?;
    let protagonist_count = protagonist_stats.item_count;
    total_stats.merge(protagonist_stats)?;

    let character_state_stats =
        ingest_character_states_inner(state, project_id, progress.as_deref_mut()).await?;
    let character_state_count = character_state_stats.item_count;
    total_stats.merge(character_state_stats)?;

    let character_attribute_event_stats =
        ingest_character_attribute_events_inner(state, project_id, progress.as_deref_mut()).await?;
    let character_attribute_event_count = character_attribute_event_stats.item_count;
    total_stats.merge(character_attribute_event_stats)?;

    let relationship_stats =
        ingest_relationships_inner(state, project_id, progress.as_deref_mut()).await?;
    let relationship_count = relationship_stats.item_count;
    total_stats.merge(relationship_stats)?;

    let scene_profile_stats =
        ingest_scene_profiles_inner(state, project_id, progress.as_deref_mut()).await?;
    let scene_profile_count = scene_profile_stats.item_count;
    total_stats.merge(scene_profile_stats)?;

    let scene_occurrence_stats =
        ingest_scene_occurrences_inner(state, project_id, progress.as_deref_mut()).await?;
    let scene_occurrence_count = scene_occurrence_stats.item_count;
    total_stats.merge(scene_occurrence_stats)?;

    let scene_transition_stats =
        ingest_scene_transitions_inner(state, project_id, progress).await?;
    let scene_transition_count = scene_transition_stats.item_count;
    total_stats.merge(scene_transition_stats)?;

    if !total_stats.errors.is_empty() {
        return Err(rag_ingest_error_message("RAG 同步", &total_stats.errors));
    }

    Ok(serde_json::json!({
        "status": "completed",
        "chapter_count": chapter_count,
        "foreshadowing_count": foreshadowing_count,
        "outline_count": outline_count,
        "part_outline_count": part_outline_count,
        "blueprint_count": blueprint_count,
        "blueprint_metadata_count": blueprint_metadata_count,
        "world_setting_count": world_setting_count,
        "character_count": character_count,
        "protagonist_count": protagonist_count,
        "character_state_count": character_state_count,
        "character_attribute_event_count": character_attribute_event_count,
        "relationship_count": relationship_count,
        "scene_profile_count": scene_profile_count,
        "scene_occurrence_count": scene_occurrence_count,
        "scene_transition_count": scene_transition_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
        "errors": [],
        "success": true,
    }))
}
