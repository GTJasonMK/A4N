//! 导出命令。
//!
//! 保持 `commands::export::*` 的 Tauri 命令入口稳定，具体导出实现按格式下沉到子模块。

macro_rules! tauri_export_read_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        $scope_expr:expr,
        $start_message:expr,
        $progress_message:expr,
        $success_message:expr,
        $error_code:expr,
        |$state:ident, $user_id:ident, $scope:ident| $run:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            $state: tauri::State<'_, a4n_runtime::AppState>,
            $user_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let $scope = $scope_expr;
            let progress = crate::progress_task::WorkflowProgressTask::new(
                window,
                $state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::ExportGeneration,
                Some($scope.project_id),
                $scope.total,
                $start_message,
            );
            progress.progress(0, $scope.total, $progress_message);
            let result = $run.await;
            if result.is_ok() {
                progress.progress($scope.total, $scope.total, $success_message);
            }
            progress.finish_result($state.inner(), &result, $error_code, |_| $success_message);
            result
        }
    };
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        $scope_expr:expr,
        $start_message:expr,
        $progress_message:expr,
        $success_message:expr,
        $error_code:expr,
        cancel: $cancel_message:expr,
        |$state:ident, $user_id:ident, $scope:ident| $run:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            $state: tauri::State<'_, a4n_runtime::AppState>,
            $user_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let $scope = $scope_expr;
            let progress = crate::progress_task::WorkflowProgressTask::new(
                window,
                $state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::ExportGeneration,
                Some($scope.project_id),
                $scope.total,
                $start_message,
            );
            let result: Result<$output, String> = async {
                progress.progress(0, $scope.total, $progress_message);
                progress.check_cancelled($cancel_message)?;
                let value = $run.await?;
                progress.check_cancelled($cancel_message)?;
                progress.progress($scope.total, $scope.total, $success_message);
                Ok(value)
            }
            .await;
            progress.finish_result($state.inner(), &result, $error_code, |_| $success_message);
            result
        }
    };
}

macro_rules! tauri_export_progress_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        $scope_expr:expr,
        $start_message:expr,
        $success_message:expr,
        $error_code:expr,
        |$state:ident, $user_id:ident, $scope:ident, $progress:ident| $run:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            $state: tauri::State<'_, a4n_runtime::AppState>,
            $user_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let $scope = $scope_expr;
            let progress_task = crate::progress_task::WorkflowProgressTask::new(
                window,
                $state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::ExportGeneration,
                Some($scope.project_id),
                $scope.total,
                $start_message,
            );
            let result = {
                let cancel_token = progress_task.cancel_token();
                let reporter = |current, total, message: &str| {
                    progress_task.progress(current, total, message)
                };
                let export_progress =
                    a4n_runtime::host_api::export_files::ExportProgress::new(
                        Some(&reporter),
                        Some(&cancel_token),
                    );
                let $progress = &export_progress;
                $run.await
            };
            progress_task.finish_result($state.inner(), &result, $error_code, |_| {
                $success_message
            });
            result
        }
    };
}

mod chapters;
mod files;
mod json_export;

pub use self::chapters::*;
pub use self::files::*;
pub use self::json_export::*;
