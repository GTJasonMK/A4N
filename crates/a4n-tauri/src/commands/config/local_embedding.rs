use crate::events::bridge::EventBridge;
use a4n_core::config::app_config::AppConfig;
use a4n_runtime::host_api::embedding_model::{
    CancelEmbeddingModelDownloadInput, DownloadProgress, EmbeddingModelDownloadInfoSummary,
    EmbeddingModelInput, LocalEmbedTextsInput, LocalEmbedTextsOutput, LocalEmbeddingConfigInput,
    LocalEmbeddingModelSummary, LocalEmbeddingProviderRuntime,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{State, Window};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct LocalEmbeddingRuntimeState {
    provider_runtime: LocalEmbeddingProviderRuntime,
    download_tasks: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl LocalEmbeddingRuntimeState {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            provider_runtime: LocalEmbeddingProviderRuntime::from_app_config(config),
            download_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn fastembed_cache_dir(&self) -> PathBuf {
        self.provider_runtime.fastembed_cache_dir()
    }
}

#[tauri::command]
pub fn local_list_embedding_models(
    state: State<'_, LocalEmbeddingRuntimeState>,
) -> Result<Vec<LocalEmbeddingModelSummary>, String> {
    a4n_runtime::host_api::embedding_model::list_local_embedding_models_in_cache(
        &state.fastembed_cache_dir(),
    )
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn local_get_embedding_model_download_info(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, String> {
    let info = a4n_runtime::host_api::embedding_model::get_embedding_model_download_info_in_cache(
        &input.model,
        &state.fastembed_cache_dir(),
    )
    .await
    .map_err(|err| err.to_string())?;
    Ok(info)
}

#[tauri::command]
pub async fn local_download_embedding_model(
    window: Window,
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: EmbeddingModelInput,
) -> Result<LocalEmbeddingModelSummary, String> {
    let task_id = input
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned);
    let cancel_token = if let Some(task_id) = task_id.as_deref() {
        let token = CancellationToken::new();
        state
            .download_tasks
            .lock()
            .await
            .insert(task_id.to_string(), token.clone());
        Some(token)
    } else {
        None
    };
    let progress_tx = task_id.as_deref().map(|task_id| {
        let task_id = task_id.to_string();
        let bridge = EventBridge::new(window.clone());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<DownloadProgress>();
        tauri::async_runtime::spawn(async move {
            while let Some(progress) = rx.recv().await {
                let _ = bridge.emit_progress_bytes(
                    &task_id,
                    progress.downloaded_bytes,
                    progress.total_bytes,
                    &progress.message,
                );
            }
        });
        tx
    });

    let result = a4n_runtime::host_api::embedding_model::download_embedding_model_to_cache(
        input,
        state.fastembed_cache_dir(),
        progress_tx,
        cancel_token,
    )
    .await;
    if let Some(task_id) = task_id.as_deref() {
        state.download_tasks.lock().await.remove(task_id);
    }
    match result {
        Ok(summary) => {
            if let Some(task_id) = task_id.as_deref() {
                let _ = EventBridge::new(window)
                    .emit_complete(task_id, Some("模型已下载并完成初始化".into()));
            }
            Ok(summary)
        }
        Err(err) => {
            let is_cancelled = matches!(&err, a4n_core::error::AppError::Cancelled);
            let message = if is_cancelled {
                "模型下载已停止".to_string()
            } else {
                err.to_string()
            };
            if let Some(task_id) = task_id.as_deref() {
                let _ = EventBridge::new(window).emit_error(
                    task_id,
                    if is_cancelled {
                        "cancelled"
                    } else {
                        "download_failed"
                    },
                    &message,
                    false,
                );
            }
            Err(message)
        }
    }
}

#[tauri::command]
pub async fn local_cancel_embedding_model_download(
    window: Window,
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: CancelEmbeddingModelDownloadInput,
) -> Result<(), String> {
    let task_id = a4n_runtime::host_api::embedding_model::required_task_id(input)
        .map_err(|err| err.to_string())?;
    let token = state.download_tasks.lock().await.remove(&task_id);
    let Some(token) = token else {
        return Err(format!("模型下载任务不存在或已结束：{task_id}"));
    };
    token.cancel();
    EventBridge::new(window)
        .emit_error(&task_id, "cancelled", "模型下载已停止", false)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn local_delete_embedding_model(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: EmbeddingModelInput,
) -> Result<(), String> {
    a4n_runtime::host_api::embedding_model::delete_local_embedding_model_in_cache(
        &input.model,
        &state.fastembed_cache_dir(),
    )
    .map_err(|err| err.to_string())?;
    state.provider_runtime.clear_provider_cache().await;
    Ok(())
}

#[tauri::command]
pub async fn local_test_embedding_config(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: LocalEmbeddingConfigInput,
) -> Result<bool, String> {
    state
        .provider_runtime
        .test_embedding_config(&input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn local_embed_texts(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: LocalEmbedTextsInput,
) -> Result<LocalEmbedTextsOutput, String> {
    state
        .provider_runtime
        .embed_texts(input)
        .await
        .map_err(|err| err.to_string())
}
