use crate::state::AppState;
use a4n_api_contract::chapter::{
    ChapterSceneSummary, ReorderChapterScenesInput, UpdateChapterSceneInput,
};
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_scene;

pub(super) fn chapter_scene_summary(scene: chapter_scene::Model) -> ChapterSceneSummary {
    ChapterSceneSummary {
        id: scene.id,
        chapter_id: scene.chapter_id,
        scene_number: scene.scene_number,
        title: scene.title,
        content: scene.content,
        word_count: scene.word_count,
        summary: scene.summary,
        scene_type: scene.scene_type,
        characters_present: scene.characters_present,
        location: scene.location,
        timeline_position: scene.timeline_position,
        transition_from_previous: scene.transition_from_previous,
        created_at: scene.created_at.to_string(),
        updated_at: scene.updated_at.to_string(),
    }
}

pub async fn list_chapter_scenes(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    let scenes = state
        .orchestrator()
        .chapter_scenes_for_user_validated(user_id, chapter_id)
        .await?;

    Ok(scenes.into_iter().map(chapter_scene_summary).collect())
}

pub async fn update_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterSceneInput,
) -> Result<ChapterSceneSummary, AppError> {
    let updated = state
        .orchestrator()
        .update_chapter_scene_for_user_validated(user_id, input)
        .await?;

    Ok(chapter_scene_summary(updated))
}

pub async fn delete_chapter_scene(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .delete_chapter_scene_for_user_validated(user_id, scene_id)
        .await?;
    Ok(())
}

pub async fn reorder_chapter_scenes(
    state: &AppState,
    user_id: i64,
    input: ReorderChapterScenesInput,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    let updated_scenes = state
        .orchestrator()
        .reorder_chapter_scenes_for_user_validated(user_id, input)
        .await?;
    Ok(updated_scenes
        .into_iter()
        .map(chapter_scene_summary)
        .collect())
}
