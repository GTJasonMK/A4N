use a4n_core::error::AppError;
use a4n_orchestrator::character::{CharacterProfileBatchProgress, CharacterProfileSyncPreviewStep};

use super::{
    CharacterAiPreviewStep, CharacterAiProgress, SyncAllCharacterProfilesInput,
    SyncAllCharacterProfilesResult,
};
use crate::{
    AppState, actions::access, prompt_renderer::RuntimePromptRenderer,
    sidecar_clients::scope_rag_operation_trace,
};

pub async fn sync_all_character_profiles_to_chapter(
    state: &AppState,
    user_id: i64,
    input: SyncAllCharacterProfilesInput,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<SyncAllCharacterProfilesResult, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let trace_port =
        super::sync_trace::RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
    let report_progress = |current: u32, total: u32, message: &str| {
        if let Some(progress) = progress {
            progress
                .report(current, total, message, "批量角色同步已停止")
                .map_err(|error| {
                    if progress.is_cancelled() {
                        AppError::Cancelled
                    } else {
                        AppError::InvalidArgument(error)
                    }
                })?;
        }
        Ok(())
    };
    scope_rag_operation_trace(
        batch_character_sync_trace_id("all", input.project_id, input.chapter_number),
        state.character().run_all_character_profile_sync_validated(
            input.project_id,
            input.chapter_number,
            &prompt_renderer,
            CharacterProfileBatchProgress {
                reporter: Some(&report_progress),
                active_task_id: progress.and_then(|progress| progress.task_id),
            },
            Some(&trace_port),
        ),
    )
    .await
    .map_err(|error| error.to_string())
}

pub(crate) async fn sync_all_character_profiles_preview_steps(
    state: &AppState,
    user_id: i64,
    input: SyncAllCharacterProfilesInput,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .character()
        .prepare_all_character_profile_sync_preview_validated(
            input.project_id,
            input.chapter_number,
            &prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())
        .map(|steps| steps.into_iter().map(preview_step).collect())
}

fn preview_step(step: CharacterProfileSyncPreviewStep) -> CharacterAiPreviewStep {
    match step.request {
        Some(request) => CharacterAiPreviewStep::llm(
            step.id,
            step.title,
            step.prompt_name,
            step.status,
            step.note,
            request,
        ),
        None => CharacterAiPreviewStep::status(
            step.id,
            step.title,
            step.prompt_name,
            step.status,
            step.note,
        ),
    }
}

fn batch_character_sync_trace_id(scope: &str, project_id: i64, chapter_number: i32) -> String {
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros());
    format!("character_profile_sync_batch:{scope}:{project_id}:{chapter_number}:{timestamp}")
}
