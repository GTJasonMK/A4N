use crate::events::bridge::EventBridge;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_engine::embedding::provider::DownloadProgress;
use a4n_runtime::AppState;
use a4n_runtime::actions::config_records::{
    self as config_records_actions, CreateEmbeddingInput, EmbeddingConfigSummary,
    ModelRequestPreview, UpdateEmbeddingInput,
};
use a4n_runtime::actions::embedding_model::{
    CancelEmbeddingModelDownloadInput, EmbeddingModelDownloadInfoSummary, EmbeddingModelInput,
    LocalEmbeddingModelSummary,
};
use std::sync::Arc;
use tauri::{State, Window};

tauri_state_command!(
    list_embedding_config,
    Vec<EmbeddingConfigSummary>,
    config_records_actions::list_embedding_config,
);
tauri_input_command!(
    create_embedding_config,
    CreateEmbeddingInput,
    EmbeddingConfigSummary,
    config_records_actions::create_embedding_config,
);
tauri_id_command!(
    set_default_embedding_config,
    (),
    config_records_actions::set_default_embedding_config,
);
tauri_id_command!(
    delete_embedding_config,
    (),
    config_records_actions::delete_embedding_config,
);
tauri_id_input_command!(
    update_embedding_config,
    UpdateEmbeddingInput,
    EmbeddingConfigSummary,
    config_records_actions::update_embedding_config,
);
tauri_id_command!(
    test_embedding_config,
    bool,
    config_records_actions::test_embedding_config,
);
tauri_id_command!(
    preview_test_embedding_config_request,
    ModelRequestPreview,
    config_records_actions::preview_test_embedding_config_request,
);

/// 列出可用的嵌入后端
#[tauri::command]
pub fn list_embedding_backends() -> Result<Vec<String>, String> {
    Ok(config_records_actions::list_embedding_backends())
}

/// 列出 fastembed 支持的本地嵌入模型及当前缓存状态。
#[tauri::command]
pub fn list_local_embedding_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalEmbeddingModelSummary>, String> {
    a4n_runtime::actions::embedding_model::list_local_embedding_models(&state)
        .map_err(|e| e.to_string())
}

/// 查询单个 fastembed 模型的远端文件大小和本地缓存进度。
#[tauri::command]
pub async fn get_embedding_model_download_info(
    state: State<'_, AppState>,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, String> {
    a4n_runtime::actions::embedding_model::get_embedding_model_download_info(&state, input)
        .await
        .map_err(|e| e.to_string())
}

/// 显式下载 fastembed 嵌入模型。该命令会真实初始化模型并写入应用内模型缓存。
#[tauri::command]
pub async fn download_embedding_model(
    window: Window,
    state: State<'_, AppState>,
    input: EmbeddingModelInput,
) -> Result<LocalEmbeddingModelSummary, String> {
    let scope = a4n_runtime::actions::embedding_model::embedding_model_download_task_scope();
    let progress_task = Arc::new(WorkflowProgressTask::new(
        window,
        state.inner(),
        input.task_id.clone(),
        TaskType::EmbeddingDownload,
        None,
        scope.total,
        scope.initial_message,
    ));
    let cancel_token = input.task_id.as_ref().map(|_| progress_task.cancel_token());
    let progress_tx = input.task_id.as_ref().map(|_| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadProgress>();
        let progress_task = progress_task.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(progress) = rx.recv().await {
                progress_task.progress_bytes(
                    progress.downloaded_bytes,
                    progress.total_bytes,
                    scope.total,
                    &progress.message,
                );
            }
        });
        tx
    });

    let summary = match a4n_runtime::actions::embedding_model::download_embedding_model(
        state.inner(),
        input.clone(),
        progress_tx,
        cancel_token,
    )
    .await
    {
        Ok(summary) => summary,
        Err(err) => {
            let is_cancelled = matches!(err, a4n_core::error::AppError::Cancelled)
                || err.to_string().contains("取消")
                || err.to_string().contains("停止");
            let msg = if is_cancelled {
                "模型下载已停止".to_string()
            } else {
                err.to_string()
            };
            let code = if is_cancelled {
                "cancelled"
            } else {
                "download_failed"
            };
            progress_task.error(code, &msg);
            progress_task.remove(state.inner());
            return Err(msg);
        }
    };
    progress_task.complete("模型已下载并完成初始化");
    progress_task.remove(state.inner());
    Ok(summary)
}

/// 取消正在进行的 fastembed 模型下载，并立刻通知前端进度弹窗进入停止态。
#[tauri::command]
pub async fn cancel_embedding_model_download(
    window: Window,
    state: State<'_, AppState>,
    input: CancelEmbeddingModelDownloadInput,
) -> Result<(), String> {
    let task_id = a4n_runtime::actions::embedding_model::required_task_id(input)
        .map_err(|e| e.to_string())?;
    if !state.task_manager.cancel(&task_id) {
        return Err(format!("模型下载任务不存在或已结束：{task_id}"));
    }

    EventBridge::new(window)
        .emit_error(&task_id, "cancelled", "模型下载已停止", false)
        .map_err(|err| err.to_string())
}

/// 删除 fastembed 本地模型缓存。
#[tauri::command]
pub fn delete_local_embedding_model(
    state: State<'_, AppState>,
    input: EmbeddingModelInput,
) -> Result<(), String> {
    a4n_runtime::actions::embedding_model::delete_local_embedding_model(&state, input)
        .map_err(|e| e.to_string())
}
