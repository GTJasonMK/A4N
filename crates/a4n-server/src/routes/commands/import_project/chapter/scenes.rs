use crate::routes::commands::{
    ApiError, chapter_scene_actions, command_input, finish_web_task, i64_arg,
    optional_string_field, register_web_task, sanitize_optional_task_id, to_value,
    web_progress_reporter,
};
use crate::state::ServerState;
use a4n_core::event::channel::TaskKind;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn list_chapter_scenes(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let chapter_id = i64_arg(args, "chapterId")?;
    Ok(json!(
        chapter_scene_actions::list_chapter_scenes(state.runtime(), user_id, chapter_id).await?
    ))
}

pub(in crate::routes::commands) async fn update_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::UpdateChapterSceneInput =
        command_input(args, "update_chapter_scene")?;
    Ok(json!(
        chapter_scene_actions::update_chapter_scene(state.runtime(), user_id, input).await?
    ))
}

pub(in crate::routes::commands) async fn delete_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let scene_id = i64_arg(args, "sceneId")?;
    chapter_scene_actions::delete_chapter_scene(state.runtime(), user_id, scene_id).await?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn reorder_chapter_scenes(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::ReorderScenesInput =
        command_input(args, "reorder_chapter_scenes")?;
    Ok(json!(
        chapter_scene_actions::reorder_chapter_scenes(state.runtime(), user_id, input).await?
    ))
}

pub(in crate::routes::commands) async fn generate_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::GenerateChapterSceneInput =
        command_input(args, "generate_chapter_scene")?;
    let scope =
        chapter_scene_actions::generate_chapter_scene_task_scope(state.runtime(), user_id, &input)
            .await
            .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备场景生成",
    );
    let progress = web_progress_reporter(task);
    let result = chapter_scene_actions::generate_chapter_scene(
        state.runtime(),
        user_id,
        input,
        &progress,
        task_id.as_deref(),
    )
    .await;
    let scene = finish_web_task(state, task_id, result)?;
    Ok(json!(scene))
}

pub(in crate::routes::commands) async fn preview_chapter_scene_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::GenerateChapterSceneInput =
        command_input(args, "preview_chapter_scene_request")?;
    let preview =
        chapter_scene_actions::preview_chapter_scene_request(state.runtime(), user_id, &input)
            .await?;
    to_value(preview)
}
