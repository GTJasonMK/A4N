use crate::AppState;
use a4n_core::error::AppError;
use a4n_orchestrator::character::{
    CharacterConflictCheckInput, CharacterConflictProgressReporter, CharacterConflictTaskScope,
};
use serde_json::Value;

pub async fn character_conflict_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<CharacterConflictTaskScope, AppError> {
    state
        .orchestrator()
        .character_conflict_task_scope_validated(
            user_id,
            &CharacterConflictCheckInput {
                project_id,
                name: name.to_string(),
                start_chapter,
                end_chapter,
            },
        )
        .await
}

pub async fn check_character_conflict(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    progress: CharacterConflictProgressReporter<'_>,
    active_task_id: Option<&str>,
) -> Result<Vec<Value>, AppError> {
    let prompt_renderer = state;
    state
        .orchestrator()
        .check_character_conflict_validated(
            user_id,
            CharacterConflictCheckInput {
                project_id,
                name: name.to_string(),
                start_chapter,
                end_chapter,
            },
            prompt_renderer,
            progress,
            active_task_id,
        )
        .await
}
