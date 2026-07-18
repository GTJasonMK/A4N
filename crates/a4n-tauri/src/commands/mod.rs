macro_rules! tauri_state_command {
    ($name:ident, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
        ) -> Result<$output, String> {
            $action(&state).await
        }
    };
}

macro_rules! tauri_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            input: $input,
        ) -> Result<$output, String> {
            $action(&state, input).await
        }
    };
}

macro_rules! tauri_id_command {
    ($name:ident, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            config_id: i64,
        ) -> Result<$output, String> {
            $action(&state, config_id).await
        }
    };
}

macro_rules! tauri_id_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            config_id: i64,
            input: $input,
        ) -> Result<$output, String> {
            $action(&state, config_id, input).await
        }
    };
}

macro_rules! tauri_image_progress_input_command {
    (
        $name:ident($input:ident: $input_ty:ty) -> $output:ty,
        $start_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $progress:ident| $scope_expr:expr,
        $run:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: tauri::Window,
            $state: tauri::State<'_, a4n_runtime::AppState>,
            $user_id: i64,
            $input: $input_ty,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope_expr;
            let progress_task = crate::progress_task::WorkflowProgressTask::new(
                window,
                $state.inner(),
                task_id,
                a4n_core::event::channel::TaskType::ImageGeneration,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let cancel_token = progress_task.cancel_token();
                let reporter =
                    |current, total, message: &str| progress_task.progress(current, total, message);
                let runtime_progress =
                    a4n_runtime::host_api::image_generation::ImageGenerationProgress::new(
                        Some(&reporter),
                        Some(&cancel_token),
                    );
                let $progress = &runtime_progress;
                $run.await
            };
            progress_task.finish_result($state.inner(), &result, "image_generation_failed", |_| {
                $success_message
            });
            result
        }
    };
}

pub mod admin;
pub mod auth;
pub mod blueprint;
pub mod canvas;
pub mod chapter;
pub mod character;
pub mod character_ai;
pub mod config;
pub mod export;
pub mod frontend_state;
pub mod galgame;
pub mod generation_analytics;
pub mod image;
pub mod import;
pub mod inspiration;
pub mod manga;
pub mod object_profile;
pub mod optimize;
pub mod outline;
pub mod portrait;
pub mod project;
pub mod prompt;
pub mod rag;
pub(crate) mod request_preview;
pub mod scene_profile;
pub mod settings_all;
pub mod source_sync;
pub mod storage;
pub mod streaming;
pub mod task;
pub mod theme;
mod validation;
pub mod video;
