use super::super::{
    CharacterAiProgress, SyncCharacterProfileInput, SyncCharacterProfileResult,
    sync_trace::RuntimeCharacterProfileSyncStageTracePort, sync_types::CharacterProgressScope,
};
use crate::{
    AppState,
    sidecar_clients::{rag_operation_timestamp, scope_rag_operation_trace},
};
use a4n_character_host_contract::{CHARACTER_PROFILE_SYNC_STEPS, CharacterProfileSyncProgress};
use a4n_error::AppError;

pub async fn sync_character_profile_from_chapters(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<SyncCharacterProfileResult, String> {
    sync_character_profile_input_with_scope(
        state,
        user_id,
        input,
        CharacterProgressScope::root(progress, CHARACTER_PROFILE_SYNC_STEPS, "角色同步已停止"),
    )
    .await
}

pub(in crate::actions::character_ai) async fn sync_character_profile_input_with_scope(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
    progress: CharacterProgressScope<'_>,
) -> Result<SyncCharacterProfileResult, String> {
    let display_name = input.name.clone();
    progress.report(
        0,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{display_name}：正在读取角色档案和可同步章节"),
    )?;
    let trace_id = character_profile_sync_rag_trace_id(&input)?;
    scope_rag_operation_trace(trace_id, async {
        let prompt_renderer = state;
        let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
        let report_progress = |current: u32, total: u32, message: &str| {
            progress.report(current, total, message).map_err(|error| {
                if progress
                    .progress
                    .is_some_and(CharacterAiProgress::is_cancelled)
                {
                    AppError::Cancelled
                } else {
                    AppError::InvalidArgument(error)
                }
            })
        };
        state
            .character_profile_sync_application()
            .sync_profile(
                user_id,
                input,
                prompt_renderer,
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

fn character_profile_sync_rag_trace_id(
    input: &SyncCharacterProfileInput,
) -> Result<String, String> {
    let subject = format!("{}:name:{}", input.project_id, input.name);
    Ok(format!(
        "character_profile_sync:{subject}:{}:{timestamp}",
        input.chapter_number,
        timestamp = rag_operation_timestamp()?,
    ))
}
