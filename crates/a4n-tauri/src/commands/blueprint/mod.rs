//! 故事蓝图命令。
//!
//! Tauri 层只保留命令签名和窗口事件；蓝图业务逻辑统一在
//! `a4n_runtime::host_api::blueprint_records`。

use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::host_api::blueprint_records as blueprint_actions;
use blueprint_actions::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary, DiffBlueprintVersionsInput,
    GenerateBlueprintInput, RefineBlueprintInput, RestoreBlueprintVersionInput,
    UpdateWorldSettingInput,
};
use tauri::State;

macro_rules! blueprint_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<$output, String> {
            $action(state.inner(), user_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

macro_rules! blueprint_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<ModelRequestPreview, String> {
            $action(state.inner(), user_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

#[tauri::command]
pub async fn get_blueprint(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Option<BlueprintSummary>, String> {
    blueprint_actions::get_blueprint(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_blueprint(
    state: State<'_, AppState>,
    user_id: i64,
    window: tauri::Window,
    input: GenerateBlueprintInput,
) -> Result<BlueprintSummary, String> {
    let task_id = input.task_id.clone();
    let has_task = task_id.is_some();
    let scope = blueprint_actions::generate_blueprint_task_scope(state.inner(), user_id, &input)
        .await
        .map_err(|e| e.to_string())?;
    let progress_task = WorkflowProgressTask::new(
        window,
        state.inner(),
        task_id,
        TaskType::LlmCall,
        Some(scope.project_id),
        scope.total,
        "启动蓝图生成",
    );
    progress_task.progress(0, scope.total, "启动蓝图生成");
    let cancel_token = has_task.then(|| progress_task.cancel_token());
    let progress = |current: u32, total: u32, message: &str| {
        progress_task.progress(current, total, message);
        Ok(())
    };
    let progress_reporter = input
        .task_id
        .as_ref()
        .map(|_| &progress as blueprint_actions::BlueprintProgressReporter<'_>);

    let result = blueprint_actions::generate_blueprint(
        state.inner(),
        user_id,
        input,
        cancel_token.as_ref(),
        progress_reporter,
    )
    .await
    .map_err(|err| match err {
        AppError::Cancelled => "蓝图生成已停止".to_string(),
        err => err.to_string(),
    });
    progress_task.finish_result(
        state.inner(),
        &result,
        "blueprint_generate_failed",
        |_| "首版蓝图已生成",
    );
    result
}

blueprint_preview_command!(
    preview_generate_blueprint_request,
    GenerateBlueprintInput,
    blueprint_actions::preview_generate_blueprint_request,
);

blueprint_input_command!(
    refine_blueprint,
    RefineBlueprintInput,
    BlueprintSummary,
    blueprint_actions::refine_blueprint,
);

blueprint_preview_command!(
    preview_refine_blueprint_request,
    RefineBlueprintInput,
    blueprint_actions::preview_refine_blueprint_request,
);

#[tauri::command]
pub async fn list_blueprint_versions(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<BlueprintVersionSummary>, String> {
    blueprint_actions::list_blueprint_versions(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

blueprint_input_command!(
    diff_blueprint_versions,
    DiffBlueprintVersionsInput,
    BlueprintDiffSummary,
    blueprint_actions::diff_blueprint_versions,
);

blueprint_input_command!(
    restore_blueprint_version,
    RestoreBlueprintVersionInput,
    BlueprintSummary,
    blueprint_actions::restore_blueprint_version,
);

blueprint_input_command!(
    update_world_setting,
    UpdateWorldSettingInput,
    serde_json::Value,
    blueprint_actions::update_world_setting,
);
