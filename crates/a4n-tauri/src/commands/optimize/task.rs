use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;

pub(super) fn optimize_progress_reporter<'a>(
    progress: &'a WorkflowProgressTask,
    cancelled_message: &'static str,
) -> impl FnMut(u32, u32, &str) -> Result<(), AppError> + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(cancelled_message)
            .map_err(AppError::InvalidArgument)
    }
}

pub(super) fn optimize_stream_reporter<'a>(
    progress: &'a WorkflowProgressTask,
    cancelled_message: &'static str,
) -> impl FnMut(&str, u32) -> Result<(), AppError> + 'a {
    move |delta, index| {
        progress.chunk(delta, index);
        progress
            .check_cancelled(cancelled_message)
            .map_err(AppError::InvalidArgument)
    }
}

macro_rules! optimize_session_stream_task_command {
    (
        $name:ident,
        $output:ty,
        $scope:path,
        $start_message:expr,
        $cancelled_message:expr,
        $error_code:expr,
        |$value:ident| $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            session_id: i64,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope(state.inner(), user_id, session_id).await?;
            let progress = $crate::progress_task::WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let mut report_progress =
                    super::task::optimize_progress_reporter(&progress, $cancelled_message);
                let mut report_stream =
                    super::task::optimize_stream_reporter(&progress, $cancelled_message);
                $action(
                    state.inner(),
                    user_id,
                    session_id,
                    Some(&mut report_progress),
                    Some(&mut report_stream),
                )
                .await
            };
            progress.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_summary
            });
            result
        }
    };
}

macro_rules! optimize_input_stream_task_command {
    (
        $name:ident,
        $input:ty,
        $output:ty,
        $scope:path,
        $start_message:expr,
        $cancelled_message:expr,
        $error_code:expr,
        |$value:ident| $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            input: $input,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope(state.inner(), user_id, &input).await?;
            let progress = $crate::progress_task::WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let mut report_progress =
                    super::task::optimize_progress_reporter(&progress, $cancelled_message);
                let mut report_stream =
                    super::task::optimize_stream_reporter(&progress, $cancelled_message);
                $action(
                    state.inner(),
                    user_id,
                    input,
                    Some(&mut report_progress),
                    Some(&mut report_stream),
                )
                .await
            };
            progress.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_summary
            });
            result
        }
    };
}

macro_rules! optimize_input_progress_task_command {
    (
        $name:ident,
        $input:ty,
        $output:ty,
        $scope:path,
        $start_message:expr,
        $cancelled_message:expr,
        $error_code:expr,
        |$value:ident| $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            input: $input,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope(state.inner(), user_id, &input).await?;
            let progress = $crate::progress_task::WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let mut report_progress =
                    super::task::optimize_progress_reporter(&progress, $cancelled_message);
                $action(state.inner(), user_id, input, Some(&mut report_progress)).await
            };
            progress.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_summary
            });
            result
        }
    };
}

pub(super) use optimize_input_progress_task_command;
pub(super) use optimize_input_stream_task_command;
pub(super) use optimize_session_stream_task_command;
