use a4n_api_contract::character::{CharacterConflictCheckInput, CharacterConflictSummary};
use a4n_character_host_contract::CHARACTER_CONFLICT_PROGRESS_TOTAL;
use a4n_error::AppError;
use a4n_orchestrator::character::validate_character_conflict_input;

use crate::{
    AppState,
    actions::{
        access,
        character_ai::{CharacterAiProgress, CharacterAiTaskScope},
    },
};

pub async fn character_conflict_task_scope(
    state: &AppState,
    user_id: i64,
    input: &CharacterConflictCheckInput,
) -> Result<CharacterAiTaskScope, String> {
    validate_character_conflict_input(input).map_err(|error| error.to_string())?;
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(CharacterAiTaskScope {
        project_id: input.project_id,
        total: CHARACTER_CONFLICT_PROGRESS_TOTAL,
    })
}

pub async fn check_character_conflict(
    state: &AppState,
    user_id: i64,
    input: CharacterConflictCheckInput,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Vec<CharacterConflictSummary>, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let reporter = |current, total, message: &str| match progress {
        Some(progress) => progress
            .report(current, total, message, "角色一致性检查已停止")
            .map_err(|error| {
                if progress.is_cancelled() {
                    AppError::Cancelled
                } else {
                    AppError::InvalidArgument(error)
                }
            }),
        None => Ok(()),
    };
    let prompt_renderer = state;
    state
        .character_conflict_application()
        .check(
            input,
            prompt_renderer,
            &reporter,
            progress.and_then(CharacterAiProgress::task_id),
        )
        .await
        .map_err(|error| error.to_string())
}
