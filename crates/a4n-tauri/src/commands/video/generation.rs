use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::video_generation::{
    self as video_generation_actions, GenerateChapterVideosBatchResult, GeneratedVideoSummary,
    PreviewChapterVideoPromptOverrideInput, ReferenceMediaVideoInput, has_active_provider_job,
};
use tauri::{State, Window};

macro_rules! tauri_video_generation_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?),
        $start_message:expr,
        $stop_message:expr,
        $error_code:expr,
        |$state:ident, $user_id:ident, $project_id:ident, $chapter_id:ident, $input:ident, $scope:ident, $task:ident, $progress:ident| {
            $($prepare:stmt);* $(;)?
        },
        $run:expr,
        $success_summary:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            $state: State<'_, AppState>,
            $user_id: i64,
            $project_id: i64,
            $chapter_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<GeneratedVideoSummary, String> {
            $($prepare;)*
            let $task = video_generation_progress_task(
                window,
                $state.inner(),
                task_id,
                $scope,
                $start_message,
            );
            let mut $progress = video_generation_progress(&$task, $stop_message);
            let result = $run;
            drop($progress);
            finish_video_generation_task(
                $task,
                $state.inner(),
                &result,
                $error_code,
                $success_summary,
            );
            result
        }
    };
}

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
    |video: &GeneratedVideoSummary| {
        if video.status == "failed" {
            video
                .error_message
                .as_deref()
                .unwrap_or("视频生成失败，后端未返回错误详情")
                .to_string()
        } else if has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            format!(
                "视频任务 #{} 已提交到供应商，正在异步生成；已保留任务 ID，可稍后轮询",
                video.id
            )
        } else {
            format!("视频 #{} 已生成并保存", video.id)
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

#[tauri::command]
pub async fn preview_chapter_video_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    config_id: Option<i64>,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ModelRequestPreview, String> {
    video_generation_actions::preview_chapter_video_request(
        state.inner(),
        video_generation_actions::GenerateChapterVideoInput {
            user_id,
            project_id,
            chapter_id,
            prompt_id,
            config_id,
        },
        prompt_override,
    )
    .await
}

#[tauri::command]
pub async fn preview_video_provider_request_by_video_id(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<ModelRequestPreview, String> {
    video_generation_actions::preview_video_provider_request_by_video_id(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        video_id,
    )
    .await
}

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
    |video: &GeneratedVideoSummary| {
        if video.status == "failed" {
            video
                .error_message
                .as_deref()
                .unwrap_or("参考媒体视频生成失败")
                .to_string()
        } else if has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
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
pub async fn preview_reference_media_video_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: ReferenceMediaVideoInput,
) -> Result<ModelRequestPreview, String> {
    video_generation_actions::preview_reference_media_video_request(
        state.inner(),
        video_generation_actions::GenerateReferenceMediaVideoInput {
            user_id,
            project_id,
            chapter_id,
            input,
        },
    )
    .await
}

#[tauri::command]
pub async fn poll_video_job(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    video_generation_actions::poll_video_job(state.inner(), user_id, video_id).await
}

fn video_generation_progress_task(
    window: Window,
    state: &AppState,
    task_id: Option<String>,
    scope: video_generation_actions::VideoGenerationTaskScope,
    message: &str,
) -> WorkflowProgressTask {
    WorkflowProgressTask::new(
        window,
        state,
        task_id,
        TaskType::VideoGeneration,
        Some(scope.project_id),
        scope.total,
        message,
    )
}

fn video_generation_progress<'a>(
    progress: &'a WorkflowProgressTask,
    stop_message: &'static str,
) -> impl FnMut(u32, u32, &str) -> Result<(), AppError> + Send + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(stop_message)
            .map_err(|_| AppError::Cancelled)
    }
}

fn finish_video_generation_task<F>(
    progress: WorkflowProgressTask,
    state: &AppState,
    result: &Result<GeneratedVideoSummary, String>,
    error_code: &str,
    success_summary: F,
) where
    F: FnOnce(&GeneratedVideoSummary) -> String,
{
    match result {
        Ok(video) if video.status == "failed" => {
            progress.error(error_code, &success_summary(video))
        }
        Ok(video) => progress.complete(success_summary(video)),
        Err(err) => progress.error(error_code, err),
    }
    progress.remove(state);
}
