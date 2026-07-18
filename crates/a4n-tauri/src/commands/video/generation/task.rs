use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::host_api::video_generation::{
    self as video_generation_actions, GeneratedVideoSummary,
};

pub(super) fn video_generation_progress_task(
    window: tauri::Window,
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

pub(super) fn video_generation_progress<'a>(
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

pub(super) fn finish_video_generation_task<F>(
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
            window: tauri::Window,
            $state: tauri::State<'_, a4n_runtime::AppState>,
            $user_id: i64,
            $project_id: i64,
            $chapter_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<a4n_runtime::host_api::video_generation::GeneratedVideoSummary, String> {
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

pub(super) use tauri_video_generation_command;
