use super::contracts::chapter_scene_summary;
use super::contracts::{
    ChapterSceneProgressReporter, ChapterSceneSummary, ChapterSceneTaskScope,
    GenerateChapterSceneInput,
};
use crate::prompt_renderer::RuntimePromptRenderer;
use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_orchestrator::chapter_scene::ChapterSceneGenerationApplicationInput;

pub async fn generate_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterSceneInput,
    progress: ChapterSceneProgressReporter<'_>,
    active_task_id: Option<&str>,
) -> Result<ChapterSceneSummary, AppError> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let scene = state
        .project_scene_records()
        .generate_chapter_scene_for_user_validated(
            user_id,
            ChapterSceneGenerationApplicationInput {
                chapter_id: input.chapter_id,
                scene_number: input.scene_number,
                scene_prompt: input.scene_prompt,
            },
            &prompt_renderer,
            progress,
            active_task_id,
        )
        .await?;
    Ok(chapter_scene_summary(&scene))
}

pub async fn generate_chapter_scene_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ChapterSceneTaskScope, AppError> {
    state
        .project_scene_records()
        .chapter_scene_task_scope_for_user_validated(
            user_id,
            &ChapterSceneGenerationApplicationInput {
                chapter_id: input.chapter_id,
                scene_number: input.scene_number,
                scene_prompt: input.scene_prompt.clone(),
            },
        )
        .await
}
