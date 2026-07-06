use super::*;

macro_rules! web_image_generation_command {
    (
        $name:ident,
        $start_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $args:ident, $progress:ident, $scope:ident| {
            $($prepare:stmt);* $(;)?
        },
        $run:expr $(,)?
    ) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $($prepare)*
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskType::ImageGeneration,
                $scope.project_id,
                $scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter($state, task, task_id.clone());
            let $progress = image_generation_actions::ImageGenerationProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
            );
            let value =
                finish_web_task($state, task_id, string_action_result($run), $success_message)
                    .await?;
            Ok(json!(value))
        }
    };
}

web_image_generation_command!(
    generate_scene_image,
    "准备生成图片",
    "图片生成完成",
    |state, user_id, args, progress, scope| {
        let project_id = i64_arg(args, "projectId")?;
        let prompt = string_arg(args, "prompt")?;
        let config_id = nullable_i64_arg(args, "configId")?;
        let chapter_id = nullable_i64_arg(args, "chapterId")?;
        let scope =
            image_generation_actions::scene_image_generation_task_scope(state, user_id, project_id)
                .await
                .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_scene_image(
        state,
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
        Some(&progress),
    )
    .await,
);

pub(super) async fn preview_scene_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let prompt = string_arg(args, "prompt")?;
    let config_id = nullable_i64_arg(args, "configId")?;
    let chapter_id = nullable_i64_arg(args, "chapterId")?;

    let preview = image_generation_actions::preview_scene_image_request(
        state, user_id, project_id, prompt, config_id, chapter_id,
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
        let scope =
            image_generation_actions::page_image_generation_task_scope(state, user_id, project_id)
                .await
                .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_page_image(
        state,
        user_id,
        project_id,
        chapter_number,
        page_number,
        Some(&progress),
    )
    .await,
);

pub(super) async fn preview_page_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "preview_page_image_request")?;
    let page_number = required_json_i32(args, "pageNumber", "preview_page_image_request")?;

    let preview = image_generation_actions::preview_page_image_request(
        state,
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
            state,
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
        state,
        user_id,
        project_id,
        requests_json,
        Some(&progress),
    )
    .await,
);

pub(super) async fn preview_batch_images_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let requests_json = required_arg(args, "requestsJson")?.clone();

    let preview = image_generation_actions::preview_batch_images_request(
        state,
        user_id,
        project_id,
        requests_json,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) async fn list_images_by_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let images = image_records_actions::list_images_by_project(state, user_id, project_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(super) async fn list_images_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::ListImagesByChapterInput =
        command_input(args, "list_images_by_chapter")?;
    let images = image_records_actions::list_images_by_chapter(
        state,
        user_id,
        input.project_id,
        input.chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(super) async fn list_video_reference_images(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::ListVideoReferenceImagesInput =
        command_input(args, "list_video_reference_images")?;
    let images = image_records_actions::list_video_reference_images(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(super) async fn list_profile_materials(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: profile_materials_actions::ListProfileMaterialsInput =
        command_input(args, "list_profile_materials")?;
    let groups = profile_materials_actions::list_profile_materials(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(groups)
}

pub(super) async fn set_profile_primary_material(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: profile_materials_actions::SetProfilePrimaryMaterialInput =
        command_input(args, "set_profile_primary_material")?;
    let group = profile_materials_actions::set_profile_primary_material(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(group)
}

pub(super) async fn upload_profile_material_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: profile_materials_actions::UploadProfileMaterialImageInput =
        command_input(args, "upload_profile_material_image")?;
    let group = profile_materials_actions::upload_profile_material_image(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(group)
}

pub(super) async fn toggle_image_favorite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    let updated = image_records_actions::toggle_image_favorite(state, user_id, image_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn update_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    let input: image_records_actions::UpdateGeneratedImageInput =
        command_input(args, "update_image")?;
    let updated = image_records_actions::update_image(state, user_id, image_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn upload_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: image_records_actions::UploadImageInput = command_input(args, "upload_image")?;
    let image = image_records_actions::upload_image(state, user_id, project_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(image)
}

pub(super) async fn delete_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    image_records_actions::delete_image(state, user_id, image_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn score_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    let score = f32_arg(args, "score")?;
    let updated = image_records_actions::score_image(state, user_id, image_id, score)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn list_videos_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let videos =
        video_records_actions::list_videos_by_chapter(state, user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(videos)
}

pub(super) async fn toggle_video_favorite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let updated = video_records_actions::toggle_video_favorite(state, user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn update_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let input: video_records_actions::UpdateGeneratedVideoInput =
        command_input(args, "update_video")?;
    let updated = video_records_actions::update_video(state, user_id, video_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn delete_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    video_records_actions::delete_video(state, user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn delete_chapter_video_assets(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let result =
        video_records_actions::delete_chapter_video_assets(state, user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(super) async fn load_video_playback_data(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let playback = video_records_actions::load_video_playback_data(state, user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(playback)
}

pub(super) async fn assemble_chapter_videos(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let preview = video_records_actions::assemble_chapter_videos(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) async fn load_cached_assembled_chapter_video_preview(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let preview = video_records_actions::load_cached_assembled_chapter_video_preview(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) async fn push_assembled_chapter_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let input: video_records_actions::PushAssembledChapterVideoInput = match args.get("input") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
            ApiError::bad_request(format!("push_assembled_chapter_video.input 无效: {e}"))
        })?,
        None => video_records_actions::PushAssembledChapterVideoInput::default(),
    };
    let result = video_records_actions::push_assembled_chapter_video(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(super) async fn update_assembly_push_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let input: video_records_actions::UpdateAssemblyPushStatusInput =
        command_input(args, "update_assembly_push_status")?;
    let video = video_records_actions::update_assembly_push_status(state, user_id, video_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(super) async fn apply_assembly_postprocess_edit(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let push_video_id = i64_arg(args, "pushVideoId")?;
    let input: video_records_actions::ApplyAssemblyPostprocessEditInput =
        command_input(args, "apply_assembly_postprocess_edit")?;
    let video = video_records_actions::apply_assembly_postprocess_edit(
        state,
        user_id,
        push_video_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(super) async fn upload_chapter_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let input: video_records_actions::UploadChapterVideoInput =
        command_input(args, "upload_chapter_video")?;
    let video =
        video_records_actions::upload_chapter_video(state, user_id, project_id, chapter_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(super) async fn upload_reference_media_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let input: video_records_actions::UploadReferenceMediaVideoInput =
        command_input(args, "upload_reference_media_video")?;
    let video = video_records_actions::upload_reference_media_video(
        state, user_id, project_id, chapter_id, input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(super) async fn upload_assembly_insert_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let input: video_records_actions::UploadAssemblyInsertVideoInput =
        command_input(args, "upload_assembly_insert_video")?;
    let video = video_records_actions::upload_assembly_insert_video(
        state,
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(super) async fn trim_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let start_secs = f32_arg(args, "startSecs")?;
    let end_secs = f32_arg(args, "endSecs")?;
    let trimmed = video_records_actions::trim_video(state, user_id, video_id, start_secs, end_secs)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(trimmed)
}

pub(super) async fn list_chapter_video_prompts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let prompts =
        video_prompt_actions::list_chapter_video_prompts(state, user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(prompts)
}

macro_rules! video_prompt_web_task_command {
    ($name:ident, $start_message:expr, $success_message:expr, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input = video_prompt_input_from_args(user_id, args, stringify!($name))?;
            let scope = video_prompt_actions::chapter_video_prompt_task_scope(state, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task
                .as_ref()
                .map(|handle| handle.cancel_token.clone())
                .unwrap_or_else(tokio_util::sync::CancellationToken::new);
            let mut progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let mut chunk_reporter = web_chunk_reporter(state, task, task_id.clone());
            let mut chunks = move |content: &str, index: u32| {
                let _ = chunk_reporter(content, index);
            };
            let result = $action(
                state,
                input,
                Some(&mut progress),
                Some(&mut chunks),
                cancel_token,
            )
            .await;
            let prompts = finish_web_task(
                state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(json!(prompts))
        }
    };
}

video_prompt_web_task_command!(
    generate_chapter_video_shot_analysis,
    "正在准备镜头分析生成",
    "镜头分析生成完成",
    video_prompt_actions::generate_chapter_video_shot_analysis,
);

video_prompt_web_task_command!(
    generate_chapter_video_segment_plan,
    "正在准备片段规划生成",
    "片段规划生成完成",
    video_prompt_actions::generate_chapter_video_segment_plan,
);

video_prompt_web_task_command!(
    generate_chapter_video_frame_prompts,
    "正在准备边界帧提示词生成",
    "边界帧提示词生成完成",
    video_prompt_actions::generate_chapter_video_frame_prompts,
);

video_prompt_web_task_command!(
    generate_chapter_video_prompts,
    "正在准备视频提示词生成",
    "视频提示词生成完成",
    video_prompt_actions::generate_chapter_video_prompts,
);

pub(super) async fn preview_chapter_video_frame_prompt_requests(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input =
        video_prompt_input_from_args(user_id, args, "preview_chapter_video_frame_prompt_requests")?;
    let preview =
        video_prompt_actions::preview_chapter_video_frame_prompt_model_request(state, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) async fn preview_chapter_video_prompt_requests(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input =
        video_prompt_input_from_args(user_id, args, "preview_chapter_video_prompt_requests")?;
    let preview = video_prompt_actions::preview_chapter_video_prompt_model_request(state, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) fn video_prompt_input_from_args(
    user_id: i64,
    args: &Value,
    command: &str,
) -> Result<video_prompt_actions::GenerateChapterVideoPromptInput, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let options = required_arg(args, "options")?;
    Ok(video_prompt_actions::GenerateChapterVideoPromptInput {
        user_id,
        project_id,
        chapter_id,
        options: video_prompt_actions::GeneratePromptOptionsInput {
            workflow_mode: optional_string_field(options, "workflow_mode")?,
            style: string_arg(options, "style")?,
            prompt_output_language: optional_string_field(options, "prompt_output_language")?,
            prompt_batch_size: required_json_i32(options, "prompt_batch_size", command)?,
            duration_secs: required_json_i32(options, "duration_secs", command)?,
            duration_secs_options: optional_json_i32_vec(
                options,
                "duration_secs_options",
                command,
            )?,
            aspect_ratio: string_arg(options, "aspect_ratio")?,
            reference_image_ids: json_i64_vec(options, "reference_image_ids", command)?,
            video_library_image_ids: optional_json_i64_vec(
                options,
                "video_library_image_ids",
                command,
            )?,
            regenerate_existing: optional_bool_field(options, "regenerate_existing")?,
        },
    })
}

pub(super) fn json_i64_vec(value: &Value, key: &str, label: &str) -> Result<Vec<i64>, ApiError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 缺失或不是数组")))?;
    array
        .iter()
        .map(|item| {
            item.as_i64()
                .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 必须是整数数组")))
        })
        .collect()
}

pub(super) fn optional_json_i64_vec(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i64>, ApiError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(_)) => json_i64_vec(value, key, label),
        Some(_) => Err(ApiError::bad_request(format!(
            "{label}.{key} 必须是整数数组或 null"
        ))),
    }
}

pub(super) fn json_i32_vec(value: &Value, key: &str, label: &str) -> Result<Vec<i32>, ApiError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 缺失或不是数组")))?;
    array
        .iter()
        .map(|item| {
            let raw = item
                .as_i64()
                .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 必须是整数数组")))?;
            i32::try_from(raw).map_err(|_| {
                ApiError::bad_request(format!("{label}.{key} 包含超出 i32 范围的整数"))
            })
        })
        .collect()
}

pub(super) fn optional_json_i32_vec(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i32>, ApiError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(_)) => json_i32_vec(value, key, label),
        Some(_) => Err(ApiError::bad_request(format!(
            "{label}.{key} 必须是整数数组或 null"
        ))),
    }
}

pub(super) async fn update_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let input = required_arg(args, "input")?;
    let visual_prompt = optional_string_field(input, "visual_prompt")?;
    let motion_prompt = nullable_string_update_field(input, "motion_prompt")?;
    let negative_prompt = nullable_string_update_field(input, "negative_prompt")?;
    let duration_secs = optional_i32_field(input, "duration_secs")?;
    let reference_image_ids_json = nullable_value_update_field(input, "reference_image_ids_json")?;
    let downstream_strategy = DownstreamConflictStrategy::from_optional_str(
        optional_string_field(input, "downstream_strategy")?.as_deref(),
    )
    .map_err(ApiError::bad_request)?;
    let updated = video_prompt_actions::update_chapter_video_prompt(
        state,
        user_id,
        prompt_id,
        video_prompt_actions::UpdateChapterVideoPromptInput {
            visual_prompt,
            motion_prompt,
            negative_prompt,
            duration_secs,
            reference_image_ids_json,
            downstream_strategy,
        },
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn optimize_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let input: video_prompt_actions::OptimizeChapterVideoPromptInput =
        command_input(args, "optimize_chapter_video_prompt")?;
    let updated =
        video_prompt_actions::optimize_chapter_video_prompt(state, user_id, prompt_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn delete_chapter_video_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let downstream_strategy = downstream_strategy_arg(args)?;
    video_prompt_actions::delete_chapter_video_prompt(
        state,
        user_id,
        prompt_id,
        downstream_strategy,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn delete_chapter_video_prompt_group(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let group = string_arg(args, "group")?;
    let downstream_strategy = downstream_strategy_arg(args)?;
    let result = video_prompt_actions::delete_chapter_video_prompt_group(
        state,
        user_id,
        project_id,
        chapter_id,
        &group,
        downstream_strategy,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(result)
}

macro_rules! video_generation_web_task_command {
    (
        $name:ident,
        $start_message:expr,
        $error_code:expr,
        |$state:ident, $user_id:ident, $args:ident, $input:ident, $scope:ident, $progress:ident, $cancel_token:ident| {
            $($prepare:stmt);* $(;)?
        },
        $run:expr,
        $success_summary:expr $(,)?
    ) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $($prepare)*
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskType::VideoGeneration,
                $scope.project_id,
                $scope.total,
                $start_message,
            )
            .await;
            let $cancel_token = task
                .as_ref()
                .map(|handle| handle.cancel_token.clone())
                .unwrap_or_else(tokio_util::sync::CancellationToken::new);
            let mut $progress = web_progress_reporter($state, task, task_id.clone());
            let result = $run;
            let video = finish_web_video_generation_task(
                $state,
                task_id,
                result,
                $error_code,
                $success_summary,
            )
            .await?;
            Ok(json!(video))
        }
    };
}

video_generation_web_task_command!(
    generate_chapter_video,
    "正在准备视频生成",
    "video_generation_failed",
    |state, user_id, args, input, scope, progress, cancel_token| {
        let input = chapter_video_generation_input(user_id, args)?;
        let scope = video_generation_actions::chapter_video_generation_task_scope(state, &input)
            .await
            .map_err(ApiError::bad_request)?;
    },
    video_generation_actions::generate_chapter_video(
        state,
        input,
        Some(&mut progress),
        cancel_token,
    )
    .await,
    |video: &video_generation_actions::GeneratedVideoSummary| {
        if video.status == "failed" {
            video
                .error_message
                .as_deref()
                .unwrap_or("视频生成失败，后端未返回错误详情")
                .to_string()
        } else if video_generation_actions::has_active_provider_job(
            video.provider_job_id.as_deref(),
            &video.status,
        ) {
            format!(
                "视频任务 #{} 已提交到供应商，正在异步生成；已保留任务 ID，可稍后轮询",
                video.id
            )
        } else {
            format!("视频 #{} 已生成并保存", video.id)
        }
    },
);

pub(super) async fn generate_chapter_videos_batch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let prompt_ids = json_i64_vec(args, "promptIds", "generate_chapter_videos_batch")?;
    let config_id = nullable_i64_arg(args, "configId")?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let input = video_generation_actions::GenerateChapterVideosBatchInput {
        user_id,
        project_id,
        chapter_id,
        prompt_ids,
        config_id,
    };
    let scope = video_generation_actions::chapter_videos_batch_generation_task_scope(state, &input)
        .await
        .map_err(ApiError::bad_request)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::VideoGeneration,
        scope.project_id,
        scope.total,
        "正在准备批量视频生成",
    )
    .await;
    let cancel_token = task
        .as_ref()
        .map(|handle| handle.cancel_token.clone())
        .unwrap_or_else(tokio_util::sync::CancellationToken::new);
    let mut progress = web_progress_reporter(state, task, task_id.clone());
    let result = video_generation_actions::generate_chapter_videos_batch(
        state,
        input,
        Some(&mut progress),
        cancel_token,
    )
    .await
    .map_err(|message| {
        if message.contains("已停止") || message.contains("已取消") {
            a4n_core::error::AppError::Cancelled
        } else {
            a4n_core::error::AppError::InvalidArgument(message)
        }
    });
    let value = finish_web_task(state, task_id, result, "批量视频生成完成").await?;
    to_value(value)
}

pub(super) async fn poll_video_job(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let updated = video_generation_actions::poll_video_job(state, user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json!(updated))
}

video_generation_web_task_command!(
    generate_reference_media_video,
    "正在准备参考媒体视频生成",
    "reference_media_video_generation_failed",
    |state, user_id, args, input, scope, progress, cancel_token| {
        let input = reference_media_video_generation_input(
            user_id,
            args,
            "generate_reference_media_video",
        )?;
        let scope =
            video_generation_actions::reference_media_video_generation_task_scope(state, &input)
                .await
                .map_err(ApiError::bad_request)?;
    },
    video_generation_actions::generate_reference_media_video(
        state,
        input,
        Some(&mut progress),
        cancel_token,
    )
    .await,
    |video: &video_generation_actions::GeneratedVideoSummary| {
        if video.status == "failed" {
            video
                .error_message
                .as_deref()
                .unwrap_or("参考媒体视频生成失败")
                .to_string()
        } else if video_generation_actions::has_active_provider_job(
            video.provider_job_id.as_deref(),
            &video.status,
        ) {
            format!(
                "参考媒体视频任务 #{} 已提交到供应商，正在异步生成；已保留任务 ID，可稍后轮询",
                video.id
            )
        } else {
            "参考媒体视频生成完成".to_string()
        }
    },
);

pub(super) async fn preview_chapter_video_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = chapter_video_generation_input(user_id, args)?;
    let prompt_override = optional_prompt_override(args, "preview_chapter_video_request")?;
    let preview =
        video_generation_actions::preview_chapter_video_request(state, input, prompt_override)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) async fn preview_video_provider_request_by_video_id(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let video_id = i64_arg(args, "videoId")?;
    let preview = video_generation_actions::preview_video_provider_request_by_video_id(
        state, user_id, project_id, chapter_id, video_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) async fn preview_reference_media_video_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = reference_media_video_generation_input(
        user_id,
        args,
        "preview_reference_media_video_request",
    )?;
    let preview = video_generation_actions::preview_reference_media_video_request(state, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(super) fn chapter_video_generation_input(
    user_id: i64,
    args: &Value,
) -> Result<video_generation_actions::GenerateChapterVideoInput, ApiError> {
    Ok(video_generation_actions::GenerateChapterVideoInput {
        user_id,
        project_id: i64_arg(args, "projectId")?,
        chapter_id: i64_arg(args, "chapterId")?,
        prompt_id: i64_arg(args, "promptId")?,
        config_id: nullable_i64_arg(args, "configId")?,
    })
}

pub(super) fn reference_media_video_generation_input(
    user_id: i64,
    args: &Value,
    command: &str,
) -> Result<video_generation_actions::GenerateReferenceMediaVideoInput, ApiError> {
    let input = command_input(args, command)?;
    Ok(video_generation_actions::GenerateReferenceMediaVideoInput {
        user_id,
        project_id: i64_arg(args, "projectId")?,
        chapter_id: i64_arg(args, "chapterId")?,
        input,
    })
}

pub(super) fn optional_prompt_override(
    args: &Value,
    command: &str,
) -> Result<Option<video_generation_actions::PreviewChapterVideoPromptOverrideInput>, ApiError> {
    match args.get("promptOverride") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|e| ApiError::bad_request(format!("{command}.promptOverride 无效: {e}"))),
    }
}

pub(super) async fn list_manga_panels(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let panels = manga_records_actions::list_manga_panels(state, user_id, project_id, chapter_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(panels)
}

pub(super) async fn create_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let input: manga_records_actions::CreateMangaPromptInput =
        command_input(args, "create_manga_prompt")?;
    let panel =
        manga_records_actions::create_manga_prompt(state, user_id, project_id, chapter_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(panel)
}

pub(super) async fn bind_manga_reference_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::BindReferenceImageInput =
        command_input(args, "bind_manga_reference_image")?;
    let updated = manga_records_actions::bind_manga_reference_image(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn update_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::UpdateMangaPromptInput =
        command_input(args, "update_manga_prompt")?;
    let updated = manga_records_actions::update_manga_prompt(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn optimize_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::OptimizeMangaPromptInput =
        command_input(args, "optimize_manga_prompt")?;
    let updated = manga_records_actions::optimize_manga_prompt(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(super) async fn delete_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let prompt_id = i64_arg(args, "promptId")?;
    let downstream_strategy = downstream_strategy_arg(args)?;
    manga_records_actions::delete_manga_prompt(state, user_id, prompt_id, downstream_strategy)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn delete_manga_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    manga_records_actions::delete_manga_by_chapter(state, user_id, project_id, chapter_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn get_manga_pipeline_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let status =
        manga_records_actions::get_manga_pipeline_status(state, user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(status)
}

pub(super) async fn cancel_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let cancelled =
        manga_records_actions::cancel_manga_pipeline(state, user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?;
    Ok(json!(cancelled))
}

pub(super) async fn start_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "start_manga_pipeline")?;
    let start_from_stage = manga_pipeline_actions::parse_manga_pipeline_start_stage(
        optional_string_field(args, "startFromStage")?,
    )?;
    start_or_resume_manga_pipeline(
        state,
        user_id,
        project_id,
        chapter_number,
        false,
        start_from_stage,
    )
    .await
}

pub(super) async fn resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "resume_manga_pipeline")?;
    start_or_resume_manga_pipeline(state, user_id, project_id, chapter_number, true, None).await
}

pub(super) async fn start_or_resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<a4n_services::manga::MangaPipelineStartStage>,
) -> Result<Value, ApiError> {
    manga_pipeline_actions::preflight_manga_pipeline(state, user_id, project_id, chapter_number)
        .await?;

    let task_id = format!(
        "manga-pipeline-{project_id}-{chapter_number}-{}",
        chrono::Utc::now().timestamp_millis()
    );
    let start_message = if resume_from_checkpoint {
        "续传漫画 8 步流水线"
    } else {
        "启动漫画 8 步流水线"
    };
    emit_workflow_progress(state, &task_id, 0, 8, start_message).await;

    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    let runtime_config_snapshot = a4n_runtime::runtime_config::current_snapshot();
    tokio::spawn(async move {
        a4n_runtime::runtime_config::request_scope(user_id, runtime_config_snapshot, async move {
            emit_workflow_progress(&state_clone, &task_id_clone, 1, 8, "正在运行 LLM 流水线").await;
            let result = manga_pipeline_actions::run_manga_pipeline(
                &state_clone,
                user_id,
                project_id,
                chapter_number,
                &task_id_clone,
                resume_from_checkpoint,
                start_from_stage,
            )
            .await;
            match result {
                Ok(panels) => {
                    emit_workflow_complete(
                        &state_clone,
                        &task_id_clone,
                        Some(format!(
                            "漫画流水线完成，生成 {} 页整页提示词",
                            panels.len()
                        )),
                    )
                    .await;
                }
                Err(err) => {
                    emit_workflow_error(
                        &state_clone,
                        &task_id_clone,
                        "pipeline_failed",
                        &err.to_string(),
                        false,
                    )
                    .await;
                }
            }
        })
        .await;
    });

    Ok(json!(task_id))
}

pub(super) async fn preview_start_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "preview_start_manga_pipeline_request",
    )?;
    let start_from_stage = manga_pipeline_actions::parse_manga_pipeline_start_stage(
        optional_string_field(args, "startFromStage")?,
    )?;
    preview_manga_pipeline_request(
        state,
        user_id,
        project_id,
        chapter_number,
        false,
        start_from_stage,
    )
    .await
}

pub(super) async fn preview_resume_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "preview_resume_manga_pipeline_request",
    )?;
    preview_manga_pipeline_request(state, user_id, project_id, chapter_number, true, None).await
}

pub(super) async fn preview_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<a4n_services::manga::MangaPipelineStartStage>,
) -> Result<Value, ApiError> {
    to_value(
        manga_pipeline_actions::preview_manga_pipeline_requests(
            state,
            user_id,
            project_id,
            chapter_number,
            resume_from_checkpoint,
            start_from_stage,
        )
        .await?,
    )
}

macro_rules! profile_record_web_commands {
    (
        $list_fn:ident, $create_fn:ident, $update_fn:ident, $delete_fn:ident,
        $list_input:ty, $input:ty,
        $list_action:path, $create_action:path, $update_action:path, $delete_action:path,
        $id_arg:literal, $id_var:ident $(,)?
    ) => {
        pub(super) async fn $list_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $list_input = command_input(args, stringify!($list_fn))?;
            to_value(
                $list_action(state, user_id, input.project_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(super) async fn $create_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            let input: $input = command_input(args, stringify!($create_fn))?;
            to_value(
                $create_action(state, user_id, project_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(super) async fn $update_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let $id_var = i64_arg(args, $id_arg)?;
            let input: $input = command_input(args, stringify!($update_fn))?;
            to_value(
                $update_action(state, user_id, $id_var, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }

        pub(super) async fn $delete_fn(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let $id_var = i64_arg(args, $id_arg)?;
            let downstream_strategy = downstream_strategy_arg(args)?;
            $delete_action(state, user_id, $id_var, downstream_strategy)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

profile_record_web_commands!(
    list_object_profiles,
    create_object_profile,
    update_object_profile,
    delete_object_profile,
    object_profile_records_actions::ListObjectProfilesInput,
    object_profile_records_actions::ObjectProfileInput,
    object_profile_records_actions::list_object_profiles,
    object_profile_records_actions::create_object_profile,
    object_profile_records_actions::update_object_profile,
    object_profile_records_actions::delete_object_profile,
    "objectProfileId",
    object_profile_id,
);

pub(super) async fn generate_object_profile_image_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::ObjectProfileImageInput =
        command_input(args, "generate_object_profile_image_prompt")?;
    let output =
        image_generation_actions::generate_object_profile_image_prompt(state, user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    Ok(json!(output))
}

web_image_generation_command!(
    generate_object_profile_image,
    "准备生成物体素材图",
    "物体素材图生成完成",
    |state, user_id, args, progress, scope| {
        let input: image_generation_actions::ObjectProfileImageInput =
            command_input(args, "generate_object_profile_image")?;
        let scope = image_generation_actions::object_profile_image_generation_task_scope(
            state, user_id, &input,
        )
        .await
        .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_object_profile_image(
        state,
        user_id,
        input,
        Some(&progress),
    )
    .await,
);

pub(super) async fn preview_object_profile_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::ObjectProfileImageInput =
        command_input(args, "preview_object_profile_image_request")?;
    let preview =
        image_generation_actions::preview_object_profile_image_request(state, user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}

profile_record_web_commands!(
    list_scene_profiles,
    create_scene_profile,
    update_scene_profile,
    delete_scene_profile,
    scene_profile_records_actions::ListSceneProfilesInput,
    scene_profile_records_actions::SceneProfileInput,
    scene_profile_records_actions::list_scene_profiles,
    scene_profile_records_actions::create_scene_profile,
    scene_profile_records_actions::update_scene_profile,
    scene_profile_records_actions::delete_scene_profile,
    "sceneProfileId",
    scene_profile_id,
);

pub(super) async fn generate_scene_profile_image_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::SceneProfileImageInput =
        command_input(args, "generate_scene_profile_image_prompt")?;
    let output =
        image_generation_actions::generate_scene_profile_image_prompt(state, user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    Ok(json!(output))
}

web_image_generation_command!(
    generate_scene_profile_image,
    "准备生成场景素材图",
    "场景素材图生成完成",
    |state, user_id, args, progress, scope| {
        let input: image_generation_actions::SceneProfileImageInput =
            command_input(args, "generate_scene_profile_image")?;
        let scope = image_generation_actions::scene_profile_image_generation_task_scope(
            state, user_id, &input,
        )
        .await
        .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_scene_profile_image(state, user_id, input, Some(&progress),)
        .await,
);

pub(super) async fn sync_scene_profiles(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = optional_i32_field(args, "chapterNumber")?;
    let scope =
        scene_profile_sync_actions::scene_profile_sync_task_scope(state, user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?;

    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备场景档案同步",
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let reporter = web_fire_and_forget_progress_reporter(state, task, task_id.clone());
    let progress = scene_profile_sync_actions::SceneProfileSyncProgress::new(
        Some(&reporter),
        cancel_token.as_ref(),
    );
    let result = scene_profile_sync_actions::sync_scene_profiles_for_chapters(
        state,
        user_id,
        project_id,
        chapter_number,
        Some(&progress),
    )
    .await;
    let value = finish_web_task(
        state,
        task_id,
        string_action_result(result),
        "场景档案同步完成",
    )
    .await?;
    Ok(value)
}
