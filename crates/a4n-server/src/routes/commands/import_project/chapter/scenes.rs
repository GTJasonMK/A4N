use crate::routes::commands::{
    ApiError, chapter_scene_actions, command_input, finish_web_task, i64_arg,
    optional_string_field, register_web_task, sanitize_optional_task_id, to_value,
    web_progress_reporter,
};
use crate::state::ServerState;
use a4n_api_contract::chapter::{
    CHAPTER_SCENE_PROGRESS_TOTAL, GenerateChapterSceneInput, ReorderChapterScenesInput,
    UpdateChapterSceneInput,
};
use a4n_domain::task_queue::TaskKind;
use serde_json::Value;

pub(in crate::routes::commands) async fn list_chapter_scenes(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let chapter_id = i64_arg(args, "chapterId")?;
    to_value(
        chapter_scene_actions::list_chapter_scenes(state.runtime(), user_id, chapter_id).await?,
    )
}

pub(in crate::routes::commands) async fn update_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: UpdateChapterSceneInput = command_input(args, "update_chapter_scene")?;
    to_value(chapter_scene_actions::update_chapter_scene(state.runtime(), user_id, input).await?)
}

pub(in crate::routes::commands) async fn delete_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let scene_id = i64_arg(args, "sceneId")?;
    chapter_scene_actions::delete_chapter_scene(state.runtime(), user_id, scene_id).await?;
    to_value(())
}

pub(in crate::routes::commands) async fn reorder_chapter_scenes(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: ReorderChapterScenesInput = command_input(args, "reorder_chapter_scenes")?;
    to_value(chapter_scene_actions::reorder_chapter_scenes(state.runtime(), user_id, input).await?)
}

pub(in crate::routes::commands) async fn generate_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GenerateChapterSceneInput = command_input(args, "generate_chapter_scene")?;
    let project_id =
        chapter_scene_actions::generate_chapter_scene_project_id(state.runtime(), user_id, &input)
            .await
            .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        project_id,
        CHAPTER_SCENE_PROGRESS_TOTAL,
        "正在准备场景生成",
    )?;
    let progress = web_progress_reporter(task.clone());
    let result = chapter_scene_actions::generate_chapter_scene(
        state.runtime(),
        user_id,
        input,
        &progress,
        task_id.as_deref(),
    )
    .await;
    let scene = finish_web_task(state, task.as_ref(), result)?;
    to_value(scene)
}

pub(in crate::routes::commands) async fn preview_chapter_scene_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GenerateChapterSceneInput = command_input(args, "preview_chapter_scene_request")?;
    let preview =
        chapter_scene_actions::preview_chapter_scene_request(state.runtime(), user_id, &input)
            .await?;
    to_value(preview)
}
