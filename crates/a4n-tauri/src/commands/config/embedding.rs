use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::host_api::config_records::{
    self as config_records_actions, CreateEmbeddingInput, EmbeddingConfigSummary,
    ModelRequestPreview, UpdateEmbeddingInput,
};
use a4n_runtime::host_api::embedding_model::{
    CancelEmbeddingModelDownloadInput, DownloadProgress, EmbeddingModelDownloadInfoSummary,
    EmbeddingModelInput, LocalEmbeddingModelSummary,
};
use a4n_runtime::host_api::task_records;
use std::sync::Arc;
use tauri::State;

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
pub async fn list_local_embedding_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalEmbeddingModelSummary>, String> {
    a4n_runtime::host_api::embedding_model::list_local_embedding_models(&state)
        .await
        .map_err(|e| e.to_string())
}

/// 查询单个 fastembed 模型的远端文件大小和本地缓存进度。
#[tauri::command]
pub async fn get_embedding_model_download_info(
    state: State<'_, AppState>,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, String> {
    a4n_runtime::host_api::embedding_model::get_embedding_model_download_info(&state, input)
        .await
        .map_err(|e| e.to_string())
}

/// 显式下载 fastembed 嵌入模型。该命令会真实初始化模型并写入应用内模型缓存。
#[tauri::command]
pub async fn download_embedding_model(
    state: State<'_, AppState>,
    input: EmbeddingModelInput,
) -> Result<LocalEmbeddingModelSummary, String> {
    let scope = a4n_runtime::host_api::embedding_model::embedding_model_download_task_scope();
    let progress_task = Arc::new(WorkflowProgressTask::new(
        state.inner(),
        input.task_id.clone(),
        TaskKind::EmbeddingDownload,
        None,
        scope.total,
        scope.initial_message,
    ));
    let cancel_token = input.task_id.as_ref().map(|_| progress_task.cancel_token());
    let (progress_tx, progress_worker) = if input.task_id.is_some() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadProgress>();
        let progress_task = progress_task.clone();
        let worker = tauri::async_runtime::spawn(async move {
            while let Some(progress) = rx.recv().await {
                progress_task.progress_bytes(
                    progress.downloaded_bytes,
                    progress.total_bytes,
                    scope.total,
                    &progress.message,
                );
            }
        });
        (Some(tx), Some(worker))
    } else {
        (None, None)
    };

    let mut result = a4n_runtime::host_api::embedding_model::download_embedding_model(
        state.inner(),
        input.clone(),
        progress_tx,
        cancel_token,
    )
    .await;
    if let Some(worker) = progress_worker {
        if let Err(join_error) = worker.await {
            let bridge_error = format!("模型下载进度事件任务异常退出: {join_error}");
            result = Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
                "{}",
                match result {
                    Ok(_) => bridge_error,
                    Err(error) => format!("{error}；{bridge_error}"),
                }
            )));
        }
    }
    let summary = match result {
        Ok(summary) => summary,
        Err(err) => {
            let is_cancelled = matches!(&err, a4n_core::error::AppError::Cancelled);
            let msg = if is_cancelled {
                "模型下载已停止".to_string()
            } else {
                err.to_string()
            };
            progress_task.remove(state.inner());
            return Err(msg);
        }
    };
    progress_task.remove(state.inner());
    Ok(summary)
}

/// 取消正在进行的 fastembed 模型下载。
#[tauri::command]
pub async fn cancel_embedding_model_download(
    state: State<'_, AppState>,
    input: CancelEmbeddingModelDownloadInput,
) -> Result<(), String> {
    let task_id = a4n_runtime::host_api::embedding_model::required_task_id(input)
        .map_err(|e| e.to_string())?;
    let handle = task_records::get_task_handle(state.inner(), &task_id)
        .ok_or_else(|| format!("模型下载任务不存在或已结束：{task_id}"))?;
    handle.cancel();
    Ok(())
}

/// 删除 fastembed 本地模型缓存。
#[tauri::command]
pub async fn delete_local_embedding_model(
    state: State<'_, AppState>,
    input: EmbeddingModelInput,
) -> Result<(), String> {
    a4n_runtime::host_api::embedding_model::delete_local_embedding_model(&state, input)
        .await
        .map_err(|e| e.to_string())
}
