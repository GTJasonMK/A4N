use crate::progress_task::{WorkflowProgressTask, WorkflowTaskContext};
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::host_api::video_prompts as video_prompt_actions;

pub(super) fn video_prompt_progress_task(
    state: &AppState,
    task_id: Option<String>,
    scope: video_prompt_actions::VideoPromptTaskScope,
    message: &str,
) -> WorkflowProgressTask {
    WorkflowProgressTask::new_with_context(
        state,
        task_id,
        TaskKind::LlmCall,
        WorkflowTaskContext {
            project_id: Some(scope.project_id),
            root_project_id: Some(scope.project_id),
            source_snapshot_id: None,
            task_scope: Some("video_prompt".to_string()),
        },
        scope.total,
        message,
    )
}

pub(super) fn tauri_video_prompt_progress<'a>(
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

macro_rules! tauri_video_prompt_command {
    (
        $name:ident,
        $start_message:expr,
        $stop_message:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            project_id: i64,
            chapter_id: i64,
            options: a4n_runtime::host_api::video_prompts::GeneratePromptOptionsInput,
            task_id: Option<String>,
        ) -> Result<Vec<a4n_runtime::host_api::video_prompts::ChapterVideoPromptSummary>, String> {
            let input = super::input::video_prompt_input(user_id, project_id, chapter_id, options);
            let scope = a4n_runtime::host_api::video_prompts::chapter_video_prompt_task_scope(
                state.inner(),
                &input,
            )
            .await?;
            let progress = super::task::video_prompt_progress_task(
                state.inner(),
                task_id,
                scope,
                $start_message,
            );
            let mut report_progress =
                super::task::tauri_video_prompt_progress(&progress, $stop_message);
            let result = $action(
                state.inner(),
                input,
                Some(&mut report_progress),
                None,
                progress.cancel_token(),
            )
            .await;
            drop(report_progress);
            progress.remove(state.inner());
            result
        }
    };
}

pub(super) use tauri_video_prompt_command;
