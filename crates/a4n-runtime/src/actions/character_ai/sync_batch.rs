use a4n_error::AppError;
use a4n_orchestrator::character::{CharacterProfileSyncPreviewStep, CharacterProfileSyncProgress};

use super::{CharacterAiProgress, SyncAllCharacterProfilesInput, SyncAllCharacterProfilesResult};
use crate::{
    AppState,
    sidecar_clients::{rag_operation_timestamp, scope_rag_operation_trace},
};

pub async fn sync_all_character_profiles_to_chapter(
    state: &AppState,
    user_id: i64,
    input: SyncAllCharacterProfilesInput,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<SyncAllCharacterProfilesResult, String> {
    let prompt_renderer = state;
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
        batch_character_sync_trace_id("all", input.project_id, input.chapter_number)?,
        state
            .character_profile_sync_application()
            .sync_all_profiles(
                user_id,
                input,
                prompt_renderer,
                CharacterProfileSyncProgress {
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
) -> Result<Vec<CharacterProfileSyncPreviewStep>, String> {
    let prompt_renderer = state;
    state
        .character_profile_sync_application()
        .preview_all_profiles_sync(user_id, input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())
}

fn batch_character_sync_trace_id(
    scope: &str,
    project_id: i64,
    chapter_number: i32,
) -> Result<String, String> {
    Ok(format!(
        "character_profile_sync_batch:{scope}:{project_id}:{chapter_number}:{}",
        rag_operation_timestamp()?
    ))
}
