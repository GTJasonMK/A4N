use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, TaskType, finish_web_task, i64_arg, image_generation_actions,
    nullable_i64_arg, optional_string_field, register_web_task, required_arg, required_json_i32,
    sanitize_optional_task_id, string_action_result, string_arg, to_value,
    web_fire_and_forget_progress_reporter,
};
use super::super::image_task::web_image_generation_command;

web_image_generation_command!(
    generate_scene_image,
    "准备生成图片",
    "图片生成完成",
    |state, user_id, args, progress, scope| {
        let project_id = i64_arg(args, "projectId")?;
        let prompt = string_arg(args, "prompt")?;
        let config_id = nullable_i64_arg(args, "configId")?;
        let chapter_id = nullable_i64_arg(args, "chapterId")?;
        let scope = image_generation_actions::scene_image_generation_task_scope(
            state.runtime(),
            user_id,
            project_id,
        )
        .await
        .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_scene_image(
        state.runtime(),
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
        Some(&progress),
    )
    .await,
);

pub(in crate::routes::commands) async fn preview_scene_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let prompt = string_arg(args, "prompt")?;
    let config_id = nullable_i64_arg(args, "configId")?;
    let chapter_id = nullable_i64_arg(args, "chapterId")?;

    let preview = image_generation_actions::preview_scene_image_request(
        state.runtime(),
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

web_image_generation_command!(
    generate_page_image,
    "准备生成整页漫画",
    "整页漫画生成完成",
    |state, user_id, args, progress, scope| {
        let project_id = i64_arg(args, "projectId")?;
        let chapter_number = required_json_i32(args, "chapterNumber", "generate_page_image")?;
        let page_number = required_json_i32(args, "pageNumber", "generate_page_image")?;
        let scope = image_generation_actions::page_image_generation_task_scope(
            state.runtime(),
            user_id,
            project_id,
        )
        .await
        .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_page_image(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
        page_number,
        Some(&progress),
    )
    .await,
);

pub(in crate::routes::commands) async fn preview_page_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "preview_page_image_request")?;
    let page_number = required_json_i32(args, "pageNumber", "preview_page_image_request")?;

    let preview = image_generation_actions::preview_page_image_request(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
        page_number,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

web_image_generation_command!(
    batch_generate_images,
    "准备批量生成图片",
    "批量图片生成完成",
    |state, user_id, args, progress, scope| {
        let project_id = i64_arg(args, "projectId")?;
        let requests_json = required_arg(args, "requestsJson")?.clone();
        let Some(scope) = image_generation_actions::batch_image_generation_task_scope(
            state.runtime(),
            user_id,
            project_id,
            &requests_json,
        )
        .await
        .map_err(ApiError::bad_request)?
        else {
            return Ok(json!([]));
        };
    },
    image_generation_actions::batch_generate_images(
        state.runtime(),
        user_id,
        project_id,
        requests_json,
        Some(&progress),
    )
    .await,
);

pub(in crate::routes::commands) async fn preview_batch_images_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let requests_json = required_arg(args, "requestsJson")?.clone();

    let preview = image_generation_actions::preview_batch_images_request(
        state.runtime(),
        user_id,
        project_id,
        requests_json,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}
