use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::image_generation::{
    self as image_generation_actions, GeneratedImageSummary,
};
use tauri::{State, Window};

macro_rules! tauri_image_generation_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        $start_message:expr,
        $success_summary:expr,
        |$state:ident, $user_id:ident, $scope:ident, $progress:ident| $scope_expr:expr,
        $run:expr $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            $state: State<'_, AppState>,
            $user_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let $scope = $scope_expr;
            let progress_task = WorkflowProgressTask::new(
                window,
                $state.inner(),
                task_id,
                TaskType::ImageGeneration,
                Some($scope.project_id),
                $scope.total,
                $start_message,
            );
            let result = {
                let cancel_token = progress_task.cancel_token();
                let reporter = |current, total, message: &str| {
                    progress_task.progress(current, total, message)
                };
                let runtime_progress = image_generation_actions::ImageGenerationProgress::new(
                    Some(&reporter),
                    Some(&cancel_token),
                );
                let $progress = &runtime_progress;
                $run.await
            };
            progress_task.finish_result(
                $state.inner(),
                &result,
                "image_generation_failed",
                $success_summary,
            );
            result
        }
    };
}

tauri_image_generation_command!(
    generate_scene_image(
        project_id: i64,
        prompt: String,
        config_id: Option<i64>,
        chapter_id: Option<i64>,
    ) -> GeneratedImageSummary,
    "准备生成图片",
    |_| "图片生成完成",
    |state, user_id, scope, progress| image_generation_actions::scene_image_generation_task_scope(
        state.inner(),
        user_id,
        project_id,
    )
    .await?,
    image_generation_actions::generate_scene_image(
        state.inner(),
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
        Some(progress),
    ),
);

tauri_image_generation_command!(
    generate_page_image(
        project_id: i64,
        chapter_number: i32,
        page_number: i32,
    ) -> GeneratedImageSummary,
    "准备生成整页漫画",
    |_| "整页漫画生成完成",
    |state, user_id, scope, progress| image_generation_actions::page_image_generation_task_scope(
        state.inner(),
        user_id,
        project_id,
    )
    .await?,
    image_generation_actions::generate_page_image(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        page_number,
        Some(progress),
    ),
);

tauri_image_generation_command!(
    batch_generate_images(
        project_id: i64,
        requests_json: serde_json::Value,
    ) -> Vec<GeneratedImageSummary>,
    "准备批量生成图片",
    |value: &Vec<GeneratedImageSummary>| format!("批量图片生成完成，共 {} 张", value.len()),
    |state, user_id, scope, progress| {
        let Some(scope) = image_generation_actions::batch_image_generation_task_scope(
            state.inner(),
            user_id,
            project_id,
            &requests_json,
        )
        .await?
        else {
            return Ok(Vec::new());
        };
        scope
    },
    image_generation_actions::batch_generate_images(
        state.inner(),
        user_id,
        project_id,
        requests_json,
        Some(progress),
    ),
);

#[tauri::command]
pub async fn preview_scene_image_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_scene_image_request(
        state.inner(),
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
    )
    .await
}

#[tauri::command]
pub async fn preview_page_image_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_page_image_request(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        page_number,
    )
    .await
}

#[tauri::command]
pub async fn preview_batch_images_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    requests_json: serde_json::Value,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_batch_images_request(
        state.inner(),
        user_id,
        project_id,
        requests_json,
    )
    .await
}
