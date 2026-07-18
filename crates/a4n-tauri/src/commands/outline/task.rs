use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;

const OUTLINE_CANCELLED_MESSAGE: &str = "大纲生成已停止";

fn progress_reporter<'a>(
    progress: &'a WorkflowProgressTask,
) -> impl Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(OUTLINE_CANCELLED_MESSAGE)
            .map_err(|_| AppError::Cancelled)
    }
}

macro_rules! outline_generation_command {
    (
        $name:ident,
        $input:ty,
        $output:ty,
        $scope:path,
        |$message_input:ident| $start_message:expr,
        $error_code:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            window: tauri::Window,
            input: $input,
        ) -> Result<$output, String> {
            let task_id = input.task_id.clone();
            let scope = $scope(state.inner(), user_id, &input)
                .await
                .map_err(|e| e.to_string())?;
            let start_message = {
                let $message_input = &input;
                ($start_message).to_string()
            };
            let progress_task = crate::progress_task::WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                &start_message,
            );
            let result = {
                let reporter = super::progress_reporter(&progress_task);
                let progress: Option<
                    a4n_runtime::host_api::outline_records::OutlineProgressReporter<'_>,
                > = Some(&reporter);
                $action(state.inner(), user_id, input, progress)
                    .await
                    .map_err(|err| err.to_string())
            };
            progress_task.finish_result(state.inner(), &result, $error_code, |_| $success_message);
            result
        }
    };
}

macro_rules! outline_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<crate::commands::request_preview::ModelRequestPreview, String> {
            $action(state.inner(), user_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

mod chapters;
mod parts;

pub use chapters::*;
pub use parts::*;
