use super::task::{
    finish_video_generation_task, tauri_video_generation_command, video_generation_progress,
    video_generation_progress_task,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::video_generation::{
    self as video_generation_actions, GenerateChapterVideosBatchResult, ReferenceMediaVideoInput,
};
use tauri::{State, Window};

tauri_video_generation_command!(
    generate_chapter_video(prompt_id: i64, config_id: Option<i64>),
    "正在准备视频生成",
    "视频生成已停止",
    "video_generation_failed",
    |state, user_id, project_id, chapter_id, input, scope, progress_task, report_progress| {
        let input = video_generation_actions::GenerateChapterVideoInput {
            user_id,
            project_id,
            chapter_id,
            prompt_id,
            config_id,
        };
        let scope =
            video_generation_actions::chapter_video_generation_task_scope(state.inner(), &input)
                .await?;
    },
    video_generation_actions::generate_chapter_video(
        state.inner(),
        input,
        Some(&mut report_progress),
        progress_task.cancel_token(),
    )
    .await,
    |video: &a4n_runtime::host_api::video_generation::GeneratedVideoSummary| {
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

tauri_video_generation_command!(
    generate_reference_media_video(input: ReferenceMediaVideoInput),
    "正在准备参考媒体视频生成",
    "参考媒体视频生成已停止",
    "reference_media_video_generation_failed",
    |state, user_id, project_id, chapter_id, input, scope, progress_task, report_progress| {
        let input = video_generation_actions::GenerateReferenceMediaVideoInput {
            user_id,
            project_id,
            chapter_id,
            input,
        };
        let scope = video_generation_actions::reference_media_video_generation_task_scope(
            state.inner(),
            &input,
        )
        .await?;
    },
    video_generation_actions::generate_reference_media_video(
        state.inner(),
        input,
        Some(&mut report_progress),
        progress_task.cancel_token(),
    )
    .await,
    |video: &a4n_runtime::host_api::video_generation::GeneratedVideoSummary| {
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

#[tauri::command]
pub async fn generate_chapter_videos_batch(
    window: Window,
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt_ids: Vec<i64>,
    config_id: Option<i64>,
    task_id: Option<String>,
) -> Result<GenerateChapterVideosBatchResult, String> {
    let input = video_generation_actions::GenerateChapterVideosBatchInput {
        user_id,
        project_id,
        chapter_id,
        prompt_ids,
        config_id,
    };
    let scope =
        video_generation_actions::chapter_videos_batch_generation_task_scope(state.inner(), &input)
            .await?;
    let progress_task = video_generation_progress_task(
        window,
        state.inner(),
        task_id,
        scope,
        "正在准备批量视频生成",
    );
    let mut report_progress = video_generation_progress(&progress_task, "视频批量生成已停止");
    let result = video_generation_actions::generate_chapter_videos_batch(
        state.inner(),
        input,
        Some(&mut report_progress),
        progress_task.cancel_token(),
    )
    .await;
    drop(report_progress);
    match &result {
        Ok(summary) => progress_task.complete(format!(
            "批量视频生成完成：成功 {} 条，失败 {} 条，等待供应商 {} 条",
            summary.success_count, summary.failed_count, summary.pending_count
        )),
        Err(err) => progress_task.error("video_batch_generation_failed", err),
    }
    progress_task.remove(state.inner());
    result
}
