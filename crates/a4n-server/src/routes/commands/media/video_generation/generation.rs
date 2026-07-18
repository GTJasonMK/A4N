use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, TaskType, finish_web_task, finish_web_video_generation_task, i64_arg,
    json_i64_vec, nullable_i64_arg, optional_string_field, register_web_task,
    sanitize_optional_task_id, to_value, video_generation_actions, web_progress_reporter,
};
use super::input::{chapter_video_generation_input, reference_media_video_generation_input};
use super::task::video_generation_web_task_command;

video_generation_web_task_command!(
    generate_chapter_video,
    "正在准备视频生成",
    "video_generation_failed",
    |state, user_id, args, input, scope, progress, cancel_token| {
        let input = chapter_video_generation_input(user_id, args)?;
        let scope =
            video_generation_actions::chapter_video_generation_task_scope(state.runtime(), &input)
                .await
                .map_err(ApiError::bad_request)?;
    },
    video_generation_actions::generate_chapter_video(
        state.runtime(),
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

pub(in crate::routes::commands) async fn generate_chapter_videos_batch(
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
    let scope = video_generation_actions::chapter_videos_batch_generation_task_scope(
        state.runtime(),
        &input,
    )
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
    let mut progress = web_progress_reporter(state, task.clone(), task_id.clone());
    let result = video_generation_actions::generate_chapter_videos_batch(
        state.runtime(),
        input,
        Some(&mut progress),
        cancel_token,
    )
    .await
    .map_err(|message| {
        if task.as_ref().is_some_and(|handle| handle.is_cancelled()) {
            a4n_core::error::AppError::Cancelled
        } else {
            a4n_core::error::AppError::InvalidArgument(message)
        }
    });
    let value = finish_web_task(state, task_id, result, "批量视频生成完成").await?;
    to_value(value)
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
        let scope = video_generation_actions::reference_media_video_generation_task_scope(
            state.runtime(),
            &input,
        )
        .await
        .map_err(ApiError::bad_request)?;
    },
    video_generation_actions::generate_reference_media_video(
        state.runtime(),
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
