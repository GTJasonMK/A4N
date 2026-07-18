use super::super::{
    contracts::CharacterSyncGenerationTrace,
    prelude::{
        AppState, BuildCharacterSyncWriteStepsInput, CHARACTER_PROFILE_SYNC_STEPS,
        CharacterProgressScope, CharacterSyncContext, CharacterSyncExtraction,
        CharacterSyncPersistenceInput, RecordCompletedGenerationRunInput, app_error_message,
        build_character_sync_write_step_records, chapter_state_record_inputs, classify_behaviors,
        decide_implicit_reviews, json, pending_attribute_events,
    },
};
use a4n_orchestrator::character::AppliedCharacterSyncPersistenceRecord;

pub(super) struct AppliedCharacterSync {
    pub(super) profile_id: i64,
    pub(super) name: String,
    pub(super) start_chapter: i32,
    pub(super) target_chapter: i32,
    pub(super) clear_to_chapter: i32,
    pub(super) selected_chapter_count: usize,
    pub(super) implicit_reviews_checked: usize,
    pub(super) applied: AppliedCharacterSyncPersistenceRecord,
}

pub(super) async fn apply_character_sync_records(
    state: &AppState,
    project_id: i64,
    workflow_run_id: i64,
    context: CharacterSyncContext,
    extraction: CharacterSyncExtraction,
    generation_trace: &CharacterSyncGenerationTrace,
    progress: CharacterProgressScope<'_>,
) -> Result<AppliedCharacterSync, String> {
    let CharacterSyncContext {
        profile,
        start_chapter,
        target_chapter,
        clear_to_chapter,
        selected_chapter_numbers,
        chapter_data,
        baseline_state,
    } = context;
    let CharacterSyncExtraction {
        changes,
        mut behaviors,
        deletion_candidates,
        chapter_states,
    } = extraction;
    let name = profile.name.clone();
    let profile_id = profile.id;
    let profile_last_synced_chapter = profile.last_synced_chapter;
    let selected_chapter_count = selected_chapter_numbers.len();
    let mut final_attributes = baseline_state.clone();

    progress.report(
        7,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在识别行为类型并检查隐性属性变化"),
    )?;
    let workflow_run_id = Some(workflow_run_id);
    let classification_suggestions = classify_behaviors(
        state,
        project_id,
        workflow_run_id,
        &profile.name,
        &changes,
        &mut behaviors,
        &mut final_attributes,
    )
    .await?;
    let implicit_review_decisions = decide_implicit_reviews(
        state,
        workflow_run_id,
        &profile,
        start_chapter,
        &final_attributes,
        &behaviors,
        &deletion_candidates,
    )
    .await?;

    progress.report(
        8,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在写入角色事件、状态索引和快照"),
    )?;
    let pending_attribute_events =
        pending_attribute_events(&baseline_state, &changes, &classification_suggestions)?;
    let write_steps = build_character_sync_write_step_records(BuildCharacterSyncWriteStepsInput {
        profile_id,
        chapter_numbers: &selected_chapter_numbers,
        attribute_events: &pending_attribute_events,
        behaviors: &behaviors,
        deletion_candidates: &deletion_candidates,
        implicit_review_decisions: &implicit_review_decisions,
        target_chapter,
    })?;
    let implicit_review_decisions_json =
        serde_json::to_value(&implicit_review_decisions).map_err(|err| err.to_string())?;
    let generation_run = RecordCompletedGenerationRunInput {
        project_id,
        workflow: "character_profile_sync".to_string(),
        step_key: "sync_character_profile_from_chapters".to_string(),
        title: format!("角色档案同步: {name}"),
        model_kind: "llm".to_string(),
        provider: None,
        model: None,
        prompt_template_names_json: Some(generation_trace.prompt_template_names_json.clone()),
        prompt_template_hashes_json: None,
        rendered_messages_json: Some(generation_trace.rendered_messages_json.clone()),
        rendered_prompts_json: None,
        input_variables_json: Some(json!({
            "profile_id": profile_id,
            "character_name": &name,
            "last_synced_chapter_before_run": profile_last_synced_chapter,
            "start_chapter": start_chapter,
            "target_chapter": target_chapter,
            "clear_to_chapter": clear_to_chapter,
            "selected_chapter_numbers": &selected_chapter_numbers,
            "baseline_state": &baseline_state,
            "chapter_data": &chapter_data})),
        source_refs_json: Some(json!({
            "profile": {
                "id": profile_id,
                "name": &name},
            "chapters": selected_chapter_numbers
                .iter()
                .map(|chapter_number| json!({ "chapter_number": chapter_number }))
                .collect::<Vec<_>>()})),
        media_refs_json: None,
        request_json: Some(generation_trace.request_json.clone()),
        response_summary_json: Some(json!({
            "profile_id": profile_id,
            "character_name": &name,
            "response_chars": generation_trace.response_chars})),
        targets: vec![],
    };
    let applied = state
        .orchestrator
        .apply_character_sync_persistence_record(CharacterSyncPersistenceInput {
            project_id,
            profile_id,
            profile_name: name.clone(),
            start_chapter,
            target_chapter,
            clear_to_chapter: Some(clear_to_chapter),
            selected_chapter_numbers,
            baseline_state,
            write_steps,
            behaviors,
            classification_suggestions,
            deletion_candidates,
            chapter_states: chapter_state_record_inputs(&chapter_states),
            implicit_review_decisions: implicit_review_decisions_json,
            generation_run,
        })
        .await
        .map_err(app_error_message)?;

    Ok(AppliedCharacterSync {
        profile_id,
        name,
        start_chapter,
        target_chapter,
        clear_to_chapter,
        selected_chapter_count,
        implicit_reviews_checked: implicit_review_decisions.len(),
        applied,
    })
}
