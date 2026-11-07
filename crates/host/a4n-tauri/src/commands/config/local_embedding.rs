use a4n_config::app_config::AppConfig;
use a4n_runtime::host_api::embedding_model::{
    CancelEmbeddingModelDownloadInput, DownloadEmbeddingModelInput,
    EmbeddingModelDownloadInfoSummary, EmbeddingModelInput, LocalEmbedTextsInput,
    LocalEmbedTextsOutput, LocalEmbeddingConfigInput, LocalEmbeddingModelSummary,
    LocalEmbeddingProviderRuntime,
};
use a4n_task_runtime::channel::normalize_task_id;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct LocalEmbeddingRuntimeState {
    provider_runtime: LocalEmbeddingProviderRuntime,
    download_tasks: Mutex<HashMap<String, CancellationToken>>,
}

impl LocalEmbeddingRuntimeState {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            provider_runtime: LocalEmbeddingProviderRuntime::from_app_config(config),
            download_tasks: Mutex::new(HashMap::new()),
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
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: DownloadEmbeddingModelInput,
) -> Result<LocalEmbeddingModelSummary, String> {
    let DownloadEmbeddingModelInput { model, task_id } = input;
    let task_id = normalize_task_id(&task_id).map_err(|err| err.to_string())?;
    let cancel_token = CancellationToken::new();
    match state.download_tasks.lock().await.entry(task_id.clone()) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(cancel_token.clone());
        }
        std::collections::hash_map::Entry::Occupied(_) => {
            return Err(format!("模型下载任务已存在：{task_id}"));
        }
    }
    let result = a4n_runtime::host_api::embedding_model::download_embedding_model_to_cache(
        EmbeddingModelInput { model },
        state.fastembed_cache_dir(),
        None,
        cancel_token,
    )
    .await;
    if state.download_tasks.lock().await.remove(&task_id).is_none() {
        return Err(format!("模型下载任务注册丢失：{task_id}"));
    }
    match result {
        Ok(summary) => Ok(summary),
        Err(err) => {
            let is_cancelled = matches!(&err, a4n_error::AppError::Cancelled);
            let message = if is_cancelled {
                "模型下载已停止".to_string()
            } else {
                err.to_string()
            };
            Err(message)
        }
    }
}

#[tauri::command]
pub async fn local_cancel_embedding_model_download(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: CancelEmbeddingModelDownloadInput,
) -> Result<(), String> {
    let task_id = normalize_task_id(&input.task_id).map_err(|err| err.to_string())?;
    let token = state.download_tasks.lock().await.get(&task_id).cloned();
    let Some(token) = token else {
        return Err(format!("模型下载任务不存在或已结束：{task_id}"));
    };
    token.cancel();
    Ok(())
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
) -> Result<(), String> {
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

#[cfg(test)]
mod tests;
