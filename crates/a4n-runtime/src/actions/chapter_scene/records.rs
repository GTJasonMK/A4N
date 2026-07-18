use super::access_errors::{
    required_updated_text, trimmed_optional_text, verify_chapter_access, verify_scene_access,
};
use super::contracts::{
    ChapterSceneSummary, ReorderScenesInput, UpdateChapterSceneInput, chapter_scene_summary,
};
use crate::actions::access;
use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_orchestrator::project::{ProjectChapterSceneReorderInput, ProjectChapterSceneUpdateInput};

pub async fn list_chapter_scenes(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    verify_chapter_access(state, user_id, chapter_id).await?;

    let scenes = state
        .orchestrator
        .chapter_scenes_by_chapter(chapter_id)
        .await?;

    Ok(scenes.iter().map(chapter_scene_summary).collect())
}

pub async fn update_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: UpdateChapterSceneInput,
) -> Result<ChapterSceneSummary, AppError> {
    let scene = verify_scene_access(state, user_id, input.scene_id).await?;
    let now = chrono::Utc::now().naive_utc();
    let mut update = ProjectChapterSceneUpdateInput {
        scene,
        content: None,
        title: None,
        summary: None,
        scene_type: None,
        location: None,
        timeline_position: None,
        updated_at: now,
    };

    if let Some(content) = input.content {
        update.content = Some(required_updated_text(&content, "场景正文")?);
    }
    if let Some(title) = input.title {
        update.title = Some(trimmed_optional_text(&title));
    }
    if let Some(summary) = input.summary {
        update.summary = Some(trimmed_optional_text(&summary));
    }
    if let Some(scene_type) = input.scene_type {
        update.scene_type = Some(required_updated_text(&scene_type, "场景类型")?);
    }
    if let Some(location) = input.location {
        update.location = Some(trimmed_optional_text(&location));
    }
    if let Some(timeline_position) = input.timeline_position {
        update.timeline_position = Some(trimmed_optional_text(&timeline_position));
    }

    let updated = state
        .orchestrator
        .update_chapter_scene_record(update)
        .await?;

    Ok(chapter_scene_summary(&updated))
}

pub async fn delete_chapter_scene(
    state: &AppState,
    user_id: i64,
    scene_id: i64,
) -> Result<(), AppError> {
    verify_scene_access(state, user_id, scene_id).await?;
    let rows_affected = state
        .orchestrator
        .delete_chapter_scene_record(scene_id)
        .await?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "场景不存在: scene_id={scene_id}"
        )));
    }
    Ok(())
}

pub async fn reorder_chapter_scenes(
    state: &AppState,
    user_id: i64,
    input: ReorderScenesInput,
) -> Result<Vec<ChapterSceneSummary>, AppError> {
    if input.order.is_empty() {
        return Ok(Vec::new());
    }

    let reorder_input = ProjectChapterSceneReorderInput { order: input.order };
    let scope = state
        .orchestrator
        .chapter_scene_reorder_scope(reorder_input.clone())
        .await?;
    access::verify_project_editor(state, user_id, scope.project_id).await?;

    let updated_scenes = state
        .orchestrator
        .reorder_chapter_scene_records(reorder_input)
        .await?;
    Ok(updated_scenes.iter().map(chapter_scene_summary).collect())
}
