use crate::state::AppState;
use a4n_api_contract::chapter::{
    ChapterSceneSummary, ReorderChapterScenesInput, UpdateChapterSceneInput,
};
use a4n_error::AppError;

pub async fn list_chapter_scenes(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    state
        .chapter_scene_read_application()
        .list(user_id, chapter_id)
        .await
}

pub async fn update_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterSceneInput,
) -> Result<ChapterSceneSummary, AppError> {
    state
        .chapter_scene_mutation_application()
        .update(user_id, input)
        .await
}

pub async fn delete_chapter_scene(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<(), AppError> {
    state
        .chapter_scene_mutation_application()
        .delete(user_id, scene_id)
        .await
}

pub async fn reorder_chapter_scenes(
    state: &AppState,
    user_id: i64,
    input: ReorderChapterScenesInput,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    state
        .chapter_scene_mutation_application()
        .reorder(user_id, input)
        .await
}
