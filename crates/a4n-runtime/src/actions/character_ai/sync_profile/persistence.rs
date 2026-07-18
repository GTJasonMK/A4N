mod write;

use self::write::{AppliedCharacterSync, apply_character_sync_records};
use super::{
    contracts::CharacterSyncGenerationTrace,
    prelude::{
        AppState, CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope, CharacterSyncContext,
        CharacterSyncExtraction, Value, json,
    },
};
use a4n_orchestrator::{
    character::AppliedCharacterSyncPersistenceRecord,
    rag::{CharacterSyncRagInput, CharacterSyncRagSummary},
};

pub(in crate::actions::character_ai) async fn apply_character_sync_extraction(
    state: &AppState,
    project_id: i64,
    workflow_run_id: i64,
    context: CharacterSyncContext,
    extraction: CharacterSyncExtraction,
    generation_trace: &CharacterSyncGenerationTrace,
    llm_trace_paths: Vec<String>,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    let applied = apply_character_sync_records(
        state,
        project_id,
        workflow_run_id,
        context,
        extraction,
        generation_trace,
        progress,
    )
    .await?;
    progress.report(
        9,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{}：角色数据已写入，正在同步角色 RAG", applied.name),
    )?;
    let rag_summary = state
        .orchestrator
        .sync_character_continuity_rag(CharacterSyncRagInput {
            project_id,
            profile_id: applied.profile_id,
            character_name: &applied.name,
            start_chapter: applied.start_chapter,
            clear_to_chapter: applied.clear_to_chapter,
            applied: &applied.applied,
        })
        .await;
    progress.report(
        CHARACTER_PROFILE_SYNC_STEPS,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{}：角色同步完成", applied.name),
    )?;

    let AppliedCharacterSync {
        name,
        start_chapter,
        target_chapter,
        selected_chapter_count,
        implicit_reviews_checked,
        applied,
        ..
    } = applied;
    let AppliedCharacterSyncPersistenceRecord {
        deleted_timeline: _,
        deletion_mark_rewind,
        write_summary,
        target_snapshot_hash,
        state_indexes,
    } = applied;
    let CharacterSyncRagSummary {
        deleted_state_rag,
        state_rag_indexed_count,
        state_rag_indexed,
        state_rag_error,
        deleted_attribute_event_rag,
        attribute_event_rag_indexed_count,
        attribute_event_rag_indexed,
        attribute_event_rag_error,
    } = rag_summary;

    Ok(json!({
        "status": "synced",
        "name": name,
        "chapters_analyzed": selected_chapter_count,
        "sync_range": {
            "from": start_chapter,
            "to": target_chapter},
        "synced_chapter": target_chapter,
        "changes_applied": write_summary.changes_applied,
        "behaviors_recorded": write_summary.behaviors_recorded,
        "classification_suggestions_applied": write_summary.classification_suggestions_applied,
        "deletions_marked": write_summary.deletions_marked,
        "deletions_executed": write_summary.deletions_executed,
        "deleted_deletion_marks": deletion_mark_rewind.deleted,
        "rewound_deletion_marks": deletion_mark_rewind.rewound,
        "implicit_reviews_checked": implicit_reviews_checked,
        "implicit_reviews_applied": write_summary.implicit_reviews_applied,
        "snapshot_hash": target_snapshot_hash,
        "state_indexes_upserted": state_indexes.len(),
        "deleted_state_rag": deleted_state_rag,
        "state_rag_indexed_count": state_rag_indexed_count,
        "state_rag_indexed": state_rag_indexed,
        "state_rag_error": state_rag_error,
        "deleted_attribute_event_rag": deleted_attribute_event_rag,
        "attribute_event_rag_indexed_count": attribute_event_rag_indexed_count,
        "attribute_event_rag_indexed": attribute_event_rag_indexed,
        "attribute_event_rag_error": attribute_event_rag_error,
        "llm_trace_paths": llm_trace_paths}))
}
