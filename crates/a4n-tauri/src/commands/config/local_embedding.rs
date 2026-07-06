use crate::events::bridge::EventBridge;
use a4n_core::config::app_config::AppConfig;
use a4n_engine::embedding::fastembed::FastEmbedProvider;
use a4n_engine::embedding::provider::{
    DownloadProgress, EmbeddingBackend, EmbeddingModelIdentity, EmbeddingProvider,
};
use a4n_runtime::actions::embedding_model::{
    CancelEmbeddingModelDownloadInput, EmbeddingModelDownloadInfoSummary, EmbeddingModelInput,
    LocalEmbeddingModelSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{State, Window};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct LocalEmbeddingRuntimeState {
    fastembed_cache_dir: PathBuf,
    download_tasks: Arc<Mutex<HashMap<String, CancellationToken>>>,
    providers: Arc<Mutex<HashMap<String, Arc<dyn EmbeddingProvider>>>>,
}

impl LocalEmbeddingRuntimeState {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            fastembed_cache_dir: a4n_runtime::dynamic_embedding::effective_fastembed_cache_dir(
                a4n_runtime::dynamic_embedding::default_fastembed_cache_dir(config),
            ),
            download_tasks: Arc::new(Mutex::new(HashMap::new())),
            providers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn fastembed_cache_dir(&self) -> PathBuf {
        self.fastembed_cache_dir.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingConfigInput {
    pub backend: String,
    pub model: String,
    pub dimension: usize,
    pub batch_size: Option<usize>,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalEmbedTextsInput {
    pub config: LocalEmbeddingConfigInput,
    pub texts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalEmbedTextsOutput {
    pub identity: EmbeddingModelIdentity,
    pub embeddings: Vec<Vec<f32>>,
}

#[tauri::command]
pub fn local_list_embedding_models(
    state: State<'_, LocalEmbeddingRuntimeState>,
) -> Result<Vec<LocalEmbeddingModelSummary>, String> {
    a4n_runtime::actions::embedding_model::list_local_embedding_models_in_cache(
        &state.fastembed_cache_dir(),
    )
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn local_get_embedding_model_download_info(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, String> {
    let info = a4n_runtime::actions::embedding_model::get_embedding_model_download_info_in_cache(
        &input.model,
        &state.fastembed_cache_dir(),
    )
    .await
    .map_err(|err| err.to_string())?;
    Ok(EmbeddingModelDownloadInfoSummary {
        model_name: info.model_name,
        model_code: info.model_code,
        total_size_bytes: info.total_size_bytes,
        downloaded_size_bytes: info.downloaded_size_bytes,
        missing_size_bytes: info.missing_size_bytes,
        files: info
            .files
            .into_iter()
            .map(
                |file| a4n_runtime::actions::embedding_model::EmbeddingModelDownloadFileSummary {
                    path: file.path,
                    size_bytes: file.size_bytes,
                    downloaded_size_bytes: file.downloaded_size_bytes,
                    downloaded: file.downloaded,
                },
            )
            .collect(),
    })
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

    let result = a4n_runtime::actions::embedding_model::download_embedding_model_to_cache(
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
            let message = if err.to_string().contains("取消") || err.to_string().contains("停止")
            {
                "模型下载已停止".to_string()
            } else {
                err.to_string()
            };
            if let Some(task_id) = task_id.as_deref() {
                let _ = EventBridge::new(window).emit_error(
                    task_id,
                    if message.contains("已停止") {
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
    let task_id = a4n_runtime::actions::embedding_model::required_task_id(input)
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
    a4n_runtime::actions::embedding_model::delete_local_embedding_model_in_cache(
        &input.model,
        &state.fastembed_cache_dir(),
    )
    .map_err(|err| err.to_string())?;
    state.providers.lock().await.clear();
    Ok(())
}

#[tauri::command]
pub async fn local_test_embedding_config(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: LocalEmbeddingConfigInput,
) -> Result<bool, String> {
    let provider = cached_local_embedding_provider(&state, &input).await?;
    provider
        .test_connection()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn local_embed_texts(
    state: State<'_, LocalEmbeddingRuntimeState>,
    input: LocalEmbedTextsInput,
) -> Result<LocalEmbedTextsOutput, String> {
    let identity = fastembed_identity_from_config(&state, &input.config)?;
    if input.texts.is_empty() {
        return Ok(LocalEmbedTextsOutput {
            identity,
            embeddings: Vec::new(),
        });
    }
    let provider = cached_local_embedding_provider(&state, &input.config).await?;
    let embeddings = provider
        .embed(input.texts)
        .await
        .map_err(|err| err.to_string())?;
    Ok(LocalEmbedTextsOutput {
        identity,
        embeddings,
    })
}

async fn cached_local_embedding_provider(
    state: &LocalEmbeddingRuntimeState,
    input: &LocalEmbeddingConfigInput,
) -> Result<Arc<dyn EmbeddingProvider>, String> {
    let (backend, model, dimension, batch_size, cache_dir) =
        resolve_fastembed_config(state, input)?;
    let key = local_provider_cache_key(&backend, &model, dimension, batch_size, &cache_dir);
    if let Some(provider) = state.providers.lock().await.get(&key).cloned() {
        return Ok(provider);
    }

    let provider =
        FastEmbedProvider::try_new(model, batch_size, cache_dir).map_err(|err| err.to_string())?;
    if provider.dimension() != dimension {
        return Err(format!(
            "嵌入配置维度为 {dimension}，但 fastembed 模型 {} 实际输出 {} 维",
            provider.model_name(),
            provider.dimension()
        ));
    }
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(provider);
    state.providers.lock().await.insert(key, provider.clone());
    Ok(provider)
}

fn resolve_fastembed_config(
    state: &LocalEmbeddingRuntimeState,
    input: &LocalEmbeddingConfigInput,
) -> Result<(String, String, usize, usize, PathBuf), String> {
    let backend = normalized_fastembed_backend_name(&input.backend)?;
    let model = required_text(&input.model, "模型名")?;
    let dimension = input.dimension;
    if dimension == 0 {
        return Err("向量维度必须大于 0".to_string());
    }
    let batch_size = input.batch_size.unwrap_or(32).max(1);
    let cache_dir = input
        .local_model_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| state.fastembed_cache_dir());
    Ok((backend, model, dimension, batch_size, cache_dir))
}

fn fastembed_identity_from_config(
    state: &LocalEmbeddingRuntimeState,
    input: &LocalEmbeddingConfigInput,
) -> Result<EmbeddingModelIdentity, String> {
    let (backend, model, dimension, _, _) = resolve_fastembed_config(state, input)?;
    Ok(EmbeddingModelIdentity {
        backend,
        model,
        dimension,
    })
}

fn normalized_fastembed_backend_name(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fastembed" => Ok(format!("{:?}", EmbeddingBackend::FastEmbed)),
        "local_hash" => Err(
            "远程桌面本机 Embedding 模式不支持 local_hash，请选择 fastembed 并下载本机模型"
                .to_string(),
        ),
        "api" => Err(
            "远程桌面本机 Embedding 模式不支持 API 后端，请选择 fastembed 并下载本机模型"
                .to_string(),
        ),
        "" => Err("嵌入后端不能为空".to_string()),
        other => Err(format!("不支持的嵌入后端: {other}")),
    }
}

fn local_provider_cache_key(
    backend: &str,
    model: &str,
    dimension: usize,
    batch_size: usize,
    cache_dir: &std::path::Path,
) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        backend,
        model,
        dimension,
        batch_size,
        cache_dir.to_string_lossy()
    )
}

fn required_text(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value.to_string())
    }
}
