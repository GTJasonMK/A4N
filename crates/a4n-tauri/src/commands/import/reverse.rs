use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::host_api::import_reverse::{
    ReverseEngineerInput, ReverseEngineeringTarget,
    preview_reverse_engineer_request as preview_reverse_engineer_model_request, reverse_engineer,
    reverse_engineer_and_apply, reverse_engineer_task_scope,
};
use tauri::State;

fn progress_reporter<'a>(
    progress: &'a WorkflowProgressTask,
    cancelled_message: &'static str,
) -> impl Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(cancelled_message)
            .map_err(|_| AppError::Cancelled)
    }
}

async fn run_reverse_engineer_command(
    state: State<'_, AppState>,
    user_id: i64,
    input: ReverseEngineerInput,
    task_id: Option<String>,
    target: ReverseEngineeringTarget,
    apply: bool,
) -> Result<serde_json::Value, String> {
    let scope = reverse_engineer_task_scope(state.inner(), user_id, &input, apply)
        .await
        .map_err(|e| e.to_string())?;
    let progress = WorkflowProgressTask::new(
        state.inner(),
        Some(user_id),
        task_id,
        TaskKind::LlmCall,
        Some(scope.project_id),
        scope.total,
        target.initial_message(apply),
    );
    let output = {
        let reporter = progress_reporter(&progress, target.cancelled_message());
        if apply {
            reverse_engineer_and_apply(
                state.inner(),
                user_id,
                &input,
                target,
                &reporter,
                progress.task_id(),
            )
            .await
            .map_err(|e| e.to_string())
        } else {
            reverse_engineer(
                state.inner(),
                user_id,
                &input,
                target,
                &reporter,
                progress.task_id(),
            )
            .await
            .map_err(|e| e.to_string())
        }
    };
    progress.remove(state.inner());
    output
}

async fn preview_reverse_engineer_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<ModelRequestPreview, String> {
    preview_reverse_engineer_model_request(state.inner(), user_id, &input, target)
        .await
        .map_err(|e| e.to_string())
}

macro_rules! reverse_engineer_commands {
    ($run:ident, $preview:ident, $apply:ident, $target:expr) => {
        #[tauri::command]
        pub async fn $run(
            state: State<'_, AppState>,
            user_id: i64,
            input: ReverseEngineerInput,
            task_id: Option<String>,
        ) -> Result<serde_json::Value, String> {
            run_reverse_engineer_command(state, user_id, input, task_id, $target, false).await
        }

        #[tauri::command]
        pub async fn $preview(
            state: State<'_, AppState>,
            user_id: i64,
            input: ReverseEngineerInput,
        ) -> Result<ModelRequestPreview, String> {
            preview_reverse_engineer_request(state, user_id, input, $target).await
        }

        #[tauri::command]
        pub async fn $apply(
            state: State<'_, AppState>,
            user_id: i64,
            input: ReverseEngineerInput,
            task_id: Option<String>,
        ) -> Result<serde_json::Value, String> {
            run_reverse_engineer_command(state, user_id, input, task_id, $target, true).await
        }
    };
}

reverse_engineer_commands!(
    reverse_engineer_blueprint,
    preview_reverse_engineer_blueprint_request,
    reverse_engineer_blueprint_and_apply,
    ReverseEngineeringTarget::Blueprint
);
reverse_engineer_commands!(
    reverse_engineer_outline,
    preview_reverse_engineer_outline_request,
    reverse_engineer_outline_and_apply,
    ReverseEngineeringTarget::Outline
);
reverse_engineer_commands!(
    reverse_engineer_part_outline,
    preview_reverse_engineer_part_outline_request,
    reverse_engineer_part_outline_and_apply,
    ReverseEngineeringTarget::PartOutline
);
