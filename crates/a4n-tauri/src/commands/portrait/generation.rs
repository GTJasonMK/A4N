use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::portrait_generation::{self as portrait_actions, PortraitSummary};
use tauri::{State, Window};

const IMAGE_CANCELLED_MESSAGE: &str = "图片生成已停止";

macro_rules! portrait_generation_command {
    (
        $name:ident,
        ($($field:ident : $field_ty:ty),* $(,)?),
        $input:expr,
        $output:ty,
        $scope:path,
        $start_message:expr,
        |$value:ident| $success_message:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            state: State<'_, AppState>,
            user_id: i64,
            $($field: $field_ty,)*
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let input = $input;
            let scope = $scope(state.inner(), user_id, &input).await?;
            let progress = WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                TaskType::ImageGeneration,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let mut report_progress = |current: u32, total: u32, message: &str| {
                progress.progress(current, total, message);
                progress
                    .check_cancelled(IMAGE_CANCELLED_MESSAGE)
                    .map_err(|_| AppError::Cancelled)
            };
            let result = $action(
                state.inner(),
                user_id,
                input,
                Some(&mut report_progress),
                progress.cancel_token(),
            )
            .await;
            progress.finish_result(state.inner(), &result, "image_generation_failed", |$value| {
                $success_message
            });
            result
        }
    };
}

macro_rules! portrait_preview_command {
    (
        $name:ident,
        ($($field:ident : $field_ty:ty),* $(,)?),
        $input:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            $($field: $field_ty,)*
        ) -> Result<ModelRequestPreview, String> {
            $action(state.inner(), user_id, $input).await
        }
    };
}

portrait_generation_command!(
    generate_portrait,
    (
        project_id: i64,
        character_name: String,
        style: Option<String>,
        expressions: Option<Vec<String>>,
        pose: Option<String>,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::GeneratePortraitInput {
        project_id,
        character_name,
        style,
        expressions,
        pose,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    PortraitSummary,
    portrait_actions::generate_portrait_task_scope,
    "正在准备人物三视图拆解生成",
    |_value| "人物三视图拆解生成完成",
    portrait_actions::generate_portrait,
);

portrait_generation_command!(
    regenerate_portrait,
    (
        portrait_id: i64,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::RegeneratePortraitInput {
        portrait_id,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    PortraitSummary,
    portrait_actions::regenerate_portrait_task_scope,
    "正在准备重绘人物三视图拆解",
    |_value| "人物三视图拆解重绘完成",
    portrait_actions::regenerate_portrait,
);

portrait_generation_command!(
    generate_age_variant,
    (
        portrait_id: i64,
        age: String,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::GenerateAgeVariantInput {
        portrait_id,
        age,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    PortraitSummary,
    portrait_actions::generate_age_variant_task_scope,
    "正在准备年龄变体",
    |_value| "年龄变体生成完成",
    portrait_actions::generate_age_variant,
);

portrait_generation_command!(
    generate_missing_portraits,
    (project_id: i64),
    portrait_actions::GenerateMissingPortraitsInput { project_id },
    Vec<PortraitSummary>,
    portrait_actions::generate_missing_portraits_task_scope,
    "准备补齐人物三视图拆解",
    |value| format!("人物三视图拆解补齐完成，共生成 {} 张", value.len()),
    portrait_actions::generate_missing_portraits,
);

portrait_preview_command!(
    preview_generate_portrait_request,
    (
        project_id: i64,
        character_name: String,
        style: Option<String>,
        expressions: Option<Vec<String>>,
        pose: Option<String>,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::GeneratePortraitInput {
        project_id,
        character_name,
        style,
        expressions,
        pose,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    portrait_actions::preview_generate_portrait_request,
);

portrait_preview_command!(
    preview_regenerate_portrait_request,
    (
        portrait_id: i64,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::RegeneratePortraitInput {
        portrait_id,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    portrait_actions::preview_regenerate_portrait_request,
);

portrait_preview_command!(
    preview_generate_age_variant_request,
    (
        portrait_id: i64,
        age: String,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::GenerateAgeVariantInput {
        portrait_id,
        age,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    portrait_actions::preview_generate_age_variant_request,
);

portrait_preview_command!(
    preview_generate_missing_portraits_request,
    (project_id: i64),
    portrait_actions::GenerateMissingPortraitsInput { project_id },
    portrait_actions::preview_generate_missing_portraits_request,
);
