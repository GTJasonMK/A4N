use crate::state::AppState;
use a4n_api_contract::chapter::{
    ChapterSceneProgressReporter, ChapterSceneSummary, GenerateChapterSceneInput,
};
use a4n_error::AppError;

pub async fn generate_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterSceneInput,
    progress: ChapterSceneProgressReporter<'_>,
    active_task_id: Option<&str>,
) -> Result<ChapterSceneSummary, AppError> {
    let prompt_renderer = state;
    state
        .chapter_scene_generation_application()
        .generate_for_user(user_id, input, prompt_renderer, progress, active_task_id)
        .await
}

pub async fn generate_chapter_scene_project_id(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<i64, AppError> {
    state
        .chapter_scene_generation_application()
        .project_id_for_user(user_id, input)
        .await
}
