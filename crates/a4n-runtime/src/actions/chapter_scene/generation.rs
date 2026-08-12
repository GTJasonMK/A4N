use super::records::chapter_scene_summary;
use crate::state::AppState;
use a4n_api_contract::chapter::{ChapterSceneSummary, GenerateChapterSceneInput};
use a4n_core::error::AppError;
use a4n_orchestrator::chapter_scene::ChapterSceneProgressReporter;

pub async fn generate_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterSceneInput,
    progress: ChapterSceneProgressReporter<'_>,
    active_task_id: Option<&str>,
) -> Result<ChapterSceneSummary, AppError> {
    let prompt_renderer = state;
    let scene = state
        .orchestrator()
        .generate_chapter_scene_for_user_validated(
            user_id,
            input,
            prompt_renderer,
            progress,
            active_task_id,
        )
        .await?;
    Ok(chapter_scene_summary(scene))
}

pub async fn generate_chapter_scene_project_id(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<i64, AppError> {
    state
        .orchestrator()
        .chapter_scene_project_id_for_user_validated(user_id, input)
        .await
}
