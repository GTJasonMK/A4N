use super::contracts::{
    ChapterSceneSummary, ReorderScenesInput, UpdateChapterSceneInput, chapter_scene_summary,
};
use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_orchestrator::project::{ChapterSceneUpdatePatch, ProjectChapterSceneReorderInput};

pub async fn list_chapter_scenes(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    let scenes = state
        .project_scene_records()
        .chapter_scenes_for_user_validated(user_id, chapter_id)
        .await?;

    Ok(scenes.iter().map(chapter_scene_summary).collect())
}

pub async fn update_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterSceneInput,
) -> Result<ChapterSceneSummary, AppError> {
    let updated = state
        .project_scene_records()
        .update_chapter_scene_for_user_validated(
            user_id,
            input.scene_id,
            ChapterSceneUpdatePatch {
                content: input.content,
                title: input.title,
                summary: input.summary,
                scene_type: input.scene_type,
                location: input.location,
                timeline_position: input.timeline_position,
            },
        )
        .await?;

    Ok(chapter_scene_summary(&updated))
}

pub async fn delete_chapter_scene(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<(), AppError> {
    state
        .project_scene_records()
        .delete_chapter_scene_for_user_validated(user_id, scene_id)
        .await?;
    Ok(())
}

pub async fn reorder_chapter_scenes(
    state: &AppState,
    user_id: i64,
    input: ReorderScenesInput,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    let reorder_input = ProjectChapterSceneReorderInput { order: input.order };
    let updated_scenes = state
        .project_scene_records()
        .reorder_chapter_scenes_for_user_validated(user_id, reorder_input)
        .await?;
    Ok(updated_scenes.iter().map(chapter_scene_summary).collect())
}
