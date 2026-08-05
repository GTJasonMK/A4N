use a4n_orchestrator::shooting_script::ChapterShootingScriptSourceInput;

use super::contracts::{
    ChapterShootingScriptUnitProgressView, ChapterShootingScriptView,
    GenerateChapterShootingScriptInput, contract_view, progress_contract_view,
};
use crate::AppState;

pub async fn generate_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    progress: &(dyn Fn(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send + Sync),
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
    active_task_id: Option<&str>,
) -> Result<ChapterShootingScriptView, String> {
    let resolved = state
        .shooting_script()
        .generate_chapter_shooting_script_for_user_validated(
            user_id,
            ChapterShootingScriptSourceInput {
                project_id: input.project_id,
                source_chapter_id: input.source_chapter_id,
            },
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
        .map_err(|error| error.to_string())?;
    contract_view(
        resolved.shooting_script.record,
        resolved.shooting_script.script,
    )
}

pub async fn generate_next_chapter_shooting_script_unit(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    progress: &(dyn Fn(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send + Sync),
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
    active_task_id: Option<&str>,
) -> Result<ChapterShootingScriptUnitProgressView, String> {
    let generated = state
        .shooting_script()
        .generate_next_chapter_shooting_script_unit_for_user_validated(
            user_id,
            ChapterShootingScriptSourceInput {
                project_id: input.project_id,
                source_chapter_id: input.source_chapter_id,
            },
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
        .map_err(|error| error.to_string())?;
    progress_contract_view(generated)
}
