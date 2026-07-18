use super::super::super::CharacterAiProgress;
use super::super::super::sync_context::{
    CharacterSyncAnchorMode, build_character_sync_context_by_name,
};
use super::super::super::sync_scope::{profile_sync_skip_reason, sync_skip_summary};
use super::super::super::sync_types::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope, CharacterSyncContext,
};
use super::super::persistence::apply_character_sync_extraction;
use super::super::stages::run_staged_character_sync;
use crate::AppState;
use crate::actions::access;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::{Value, json};

pub async fn sync_character_profile_from_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    chapter_number: i32,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    sync_character_profile_from_chapters_with_scope(
        state,
        project_id,
        name,
        chapter_number,
        CharacterProgressScope::root(progress, CHARACTER_PROFILE_SYNC_STEPS, "角色同步已停止"),
    )
    .await
}

pub(in crate::actions::character_ai) async fn sync_character_profile_from_chapters_with_scope(
    state: &AppState,
    project_id: i64,
    name: &str,
    chapter_number: i32,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    progress.report(
        0,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在读取角色档案和可同步章节"),
    )?;
    let context = build_character_sync_context_by_name(
        state,
        project_id,
        name,
        chapter_number,
        CharacterSyncAnchorMode::EnsureCurrent,
    )
    .await?;
    sync_character_profile_context_with_scope(state, project_id, context, progress).await
}

pub(in crate::actions::character_ai) async fn sync_character_profile_context_with_scope(
    state: &AppState,
    project_id: i64,
    context: CharacterSyncContext,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    let name = context.profile.name.clone();
    if profile_sync_skip_reason(&context.profile, context.target_chapter).is_some() {
        progress.report(
            CHARACTER_PROFILE_SYNC_STEPS,
            CHARACTER_PROFILE_SYNC_STEPS,
            &format!("{name}：无需同步到第 {} 章，跳过", context.target_chapter),
        )?;
        return Ok(sync_skip_summary(&context.profile, context.target_chapter));
    }

    let workflow_fingerprint = character_sync_workflow_fingerprint(project_id, &context);
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "character_profile_sync:{}:{}",
                context.profile.id, context.target_chapter
            ),
            workflow_kind: "character_profile_sync".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(context.profile.id),
            chapter_id: None,
            mode: Some(format!(
                "{}-{}",
                context.start_chapter, context.target_chapter
            )),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("character_sync.start".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;
    let rag_trace_id = crate::sidecar_clients::workflow_rag_operation_trace_id(workflow_run_id)?;

    let result = crate::sidecar_clients::scope_rag_operation_trace(rag_trace_id, async {
        let staged =
            run_staged_character_sync(state, project_id, workflow_run_id, &context, progress)
                .await?;
        apply_character_sync_extraction(
            state,
            project_id,
            workflow_run_id,
            context,
            staged.extraction,
            &staged.generation_trace,
            staged.llm_trace_paths,
            progress,
        )
        .await
    })
    .await;

    finalize_character_sync_workflow(state, workflow_run_id, &result).await?;
    result
}

pub(super) fn character_sync_workflow_fingerprint(
    project_id: i64,
    context: &CharacterSyncContext,
) -> Value {
    json!({
        "project_id": project_id,
        "profile_id": context.profile.id,
        "profile_name": &context.profile.name,
        "last_synced_chapter": context.profile.last_synced_chapter,
        "start_chapter": context.start_chapter,
        "target_chapter": context.target_chapter,
        "clear_to_chapter": context.clear_to_chapter,
        "selected_chapter_numbers": &context.selected_chapter_numbers,
        "baseline_state": &context.baseline_state,
        "base_attributes": &context.profile.base_attributes,
        "chapter_data": &context.chapter_data})
}

pub(super) async fn finalize_character_sync_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Value, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}
