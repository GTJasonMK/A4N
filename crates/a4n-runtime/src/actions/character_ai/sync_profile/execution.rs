use super::super::{
    CharacterAiProgress,
    sync_trace::RuntimeCharacterProfileSyncStageTracePort,
    sync_types::{CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope},
};
use crate::{
    AppState, actions::access, prompt_renderer::RuntimePromptRenderer,
    sidecar_clients::scope_rag_operation_trace,
};
use a4n_core::error::AppError;
use a4n_orchestrator::character::{
    CharacterProfileSyncApplicationInput, CharacterProfileSyncProgress, CharacterProfileSyncSubject,
};
use serde_json::Value;

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
        .map_err(|error| error.to_string())?;
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
    sync_character_profile_input_with_scope(
        state,
        CharacterProfileSyncApplicationInput::incremental_by_name(project_id, name, chapter_number),
        progress,
    )
    .await
}

pub(in crate::actions::character_ai) async fn sync_character_profile_input_with_scope(
    state: &AppState,
    input: CharacterProfileSyncApplicationInput,
    progress: CharacterProgressScope<'_>,
) -> Result<Value, String> {
    let display_name = match &input.subject {
        CharacterProfileSyncSubject::ByName { name, .. } => name.clone(),
        CharacterProfileSyncSubject::Profile(profile) => profile.name.clone(),
    };
    progress.report(
        0,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{display_name}：正在读取角色档案和可同步章节"),
    )?;
    let trace_id = character_profile_sync_rag_trace_id(&input);
    scope_rag_operation_trace(trace_id, async {
        let prompt_renderer = RuntimePromptRenderer::new(state);
        let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
        let report_progress = |current: u32, total: u32, message: &str| {
            progress.report(current, total, message).map_err(|error| {
                if progress.progress.is_some_and(CharacterAiProgress::is_cancelled) {
                    AppError::Cancelled
                } else {
                    AppError::InvalidArgument(error)
                }
            })
        };
        state
            .character()
            .run_character_profile_sync_validated(
                input,
                &prompt_renderer,
                Some(CharacterProfileSyncProgress {
                    reporter: Some(&report_progress),
                    active_task_id: progress.progress.and_then(|progress| progress.task_id),
                }),
                Some(&trace_port),
            )
            .await
            .map_err(|error| error.to_string())
    })
    .await
}

fn character_profile_sync_rag_trace_id(input: &CharacterProfileSyncApplicationInput) -> String {
    let subject = match &input.subject {
        CharacterProfileSyncSubject::ByName { project_id, name } => {
            format!("{project_id}:name:{name}")
        }
        CharacterProfileSyncSubject::Profile(profile) => {
            format!("{}:profile:{}", profile.project_id, profile.id)
        }
    };
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros());
    format!(
        "character_profile_sync:{subject}:{}:{timestamp}",
        input.target_chapter
    )
}
