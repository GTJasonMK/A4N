use super::contracts::{
    ChapterShootingScriptUnitProgressView, ChapterShootingScriptView,
    GenerateChapterShootingScriptInput,
};
use crate::AppState;

pub async fn generate_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    progress: &(dyn Fn(u32, u32, &str) -> Result<(), a4n_error::AppError> + Send + Sync),
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
    active_task_id: Option<&str>,
) -> Result<ChapterShootingScriptView, String> {
    state
        .shooting_script_full_generation_application()
        .generate(
            user_id,
            input,
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn generate_next_chapter_shooting_script_unit(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    progress: &(dyn Fn(u32, u32, &str) -> Result<(), a4n_error::AppError> + Send + Sync),
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
    active_task_id: Option<&str>,
) -> Result<ChapterShootingScriptUnitProgressView, String> {
    state
        .shooting_script_next_unit_generation_application()
        .generate_next(
            user_id,
            input,
            progress,
            stream_reporter,
            cancel_token,
            active_task_id,
        )
        .await
        .map_err(|error| error.to_string())
}
