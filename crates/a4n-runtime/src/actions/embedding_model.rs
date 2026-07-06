use crate::AppState;
use a4n_core::error::AppError;
use a4n_engine::embedding::fastembed::{
    FastEmbedDownloadInfo, FastEmbedLocalModel, delete_fastembed_model, download_fastembed_model,
    fastembed_model_download_info, list_fastembed_local_models,
};
use a4n_engine::embedding::provider::DownloadProgress;
use a4n_engine::queue::priority::{TaskPriority, TaskQueue, TaskType as QueueTaskType};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalEmbeddingModelSummary {
    pub name: String,
    pub model_code: String,
    pub description: String,
    pub dimension: usize,
    pub downloaded: bool,
    pub cache_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelDownloadInfoSummary {
    pub model_name: String,
    pub model_code: String,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub missing_size_bytes: u64,
    pub files: Vec<EmbeddingModelDownloadFileSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingModelDownloadFileSummary {
    pub path: String,
    pub size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingModelInput {
    pub model: String,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelEmbeddingModelDownloadInput {
    pub task_id: String,
}

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingModelDownloadTaskScope {
    pub total: u32,
    pub initial_message: &'static str,
}

pub fn embedding_model_download_task_scope() -> EmbeddingModelDownloadTaskScope {
    EmbeddingModelDownloadTaskScope {
        total: 1000,
        initial_message: "正在读取模型文件大小",
    }
}

pub fn list_local_embedding_models(
    state: &AppState,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    list_local_embedding_models_in_cache(&fastembed_cache_dir(state))
}

pub fn list_local_embedding_models_in_cache(
    cache_dir: &Path,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    Ok(list_fastembed_local_models(cache_dir)?
        .into_iter()
        .map(to_local_embedding_model_summary)
        .collect())
}

pub async fn get_embedding_model_download_info(
    state: &AppState,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    let info =
        get_embedding_model_download_info_in_cache(&model, &fastembed_cache_dir(state)).await?;
    Ok(to_embedding_model_download_info_summary(info))
}

pub async fn get_embedding_model_download_info_in_cache(
    model: &str,
    cache_dir: &Path,
) -> Result<FastEmbedDownloadInfo, AppError> {
    let model = required_text(model, "fastembed 模型名")?;
    fastembed_model_download_info(&model, cache_dir.to_path_buf()).await
}

pub async fn download_embedding_model(
    state: &AppState,
    input: EmbeddingModelInput,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    let cache_dir = fastembed_cache_dir(state);
    let queue_cancel_token = cancel_token.clone().unwrap_or_default();
    let model_for_download = model.clone();
    let cache_dir_for_download = cache_dir.clone();

    let download_result = run_queued_request_with_cancel(
        state,
        QueueTaskType::EmbeddingDownload,
        TaskPriority::Low,
        None,
        queue_cancel_token,
        || async move {
            download_fastembed_model(
                &model_for_download,
                cache_dir_for_download,
                progress_tx,
                cancel_token,
            )
            .await
        },
    )
    .await;
    if let Err(err) = download_result {
        return Err(normalize_download_error(err));
    }

    list_local_embedding_models(state)?
        .into_iter()
        .find(|item| {
            item.name.eq_ignore_ascii_case(&model) || item.model_code.eq_ignore_ascii_case(&model)
        })
        .ok_or_else(|| AppError::ConfigError(format!("下载完成但未能定位模型状态: {model}")))
}

pub async fn download_embedding_model_to_cache(
    input: EmbeddingModelInput,
    cache_dir: PathBuf,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    if let Err(err) =
        download_fastembed_model(&model, cache_dir.clone(), progress_tx, cancel_token).await
    {
        return Err(normalize_download_error(err));
    }
    list_local_embedding_models_in_cache(&cache_dir)?
        .into_iter()
        .find(|item| {
            item.name.eq_ignore_ascii_case(&model) || item.model_code.eq_ignore_ascii_case(&model)
        })
        .ok_or_else(|| AppError::ConfigError(format!("下载完成但未能定位模型状态: {model}")))
}

pub fn delete_local_embedding_model(
    state: &AppState,
    input: EmbeddingModelInput,
) -> Result<(), AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    delete_local_embedding_model_in_cache(&model, &fastembed_cache_dir(state))
}

pub fn delete_local_embedding_model_in_cache(
    model: &str,
    cache_dir: &Path,
) -> Result<(), AppError> {
    let model = required_text(model, "fastembed 模型名")?;
    delete_fastembed_model(&model, cache_dir)
}

pub fn required_task_id(input: CancelEmbeddingModelDownloadInput) -> Result<String, AppError> {
    required_text(&input.task_id, "模型下载任务 ID")
}

async fn run_queued_request_with_cancel<F, Fut, T>(
    state: &AppState,
    task_type: QueueTaskType,
    priority: TaskPriority,
    project_id: Option<i64>,
    cancel_token: CancellationToken,
    operation: F,
) -> Result<T, AppError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let submitted_id = state
        .task_queue
        .submit(task_type, priority, project_id)
        .await?;
    let Some(permit) = state
        .task_queue
        .acquire_task_with_cancel(&submitted_id, cancel_token)
        .await?
    else {
        if let Err(err) = state.task_queue.cancel(&submitted_id).await {
            tracing::warn!(
                task_id = %submitted_id,
                error = %err,
                "同步取消队列任务失败"
            );
        }
        return Err(AppError::Cancelled);
    };
    let running_id = permit.task_id().to_string();

    match operation().await {
        Ok(value) => {
            permit.complete();
            if let Err(err) = state.task_queue.mark_done(&running_id).await {
                tracing::warn!(
                    task_id = %running_id,
                    error = %err,
                    "同步队列任务完成状态失败"
                );
            }
            Ok(value)
        }
        Err(err) => {
            let message = err.to_string();
            permit.fail(message.clone());
            if let Err(status_err) = state.task_queue.mark_failed(&running_id, &message).await {
                tracing::warn!(
                    task_id = %running_id,
                    error = %status_err,
                    "同步队列任务失败状态失败"
                );
            }
            Err(err)
        }
    }
}

fn fastembed_cache_dir(state: &AppState) -> PathBuf {
    crate::dynamic_embedding::effective_fastembed_cache_dir(
        crate::dynamic_embedding::default_fastembed_cache_dir(&state.config),
    )
}

fn to_local_embedding_model_summary(model: FastEmbedLocalModel) -> LocalEmbeddingModelSummary {
    LocalEmbeddingModelSummary {
        name: model.name,
        model_code: model.model_code,
        description: model.description,
        dimension: model.dimension,
        downloaded: model.downloaded,
        cache_path: model.cache_path.to_string_lossy().to_string(),
        size_bytes: model.size_bytes,
    }
}

fn to_embedding_model_download_info_summary(
    info: FastEmbedDownloadInfo,
) -> EmbeddingModelDownloadInfoSummary {
    EmbeddingModelDownloadInfoSummary {
        model_name: info.model_name,
        model_code: info.model_code,
        total_size_bytes: info.total_size_bytes,
        downloaded_size_bytes: info.downloaded_size_bytes,
        missing_size_bytes: info.missing_size_bytes,
        files: info
            .files
            .into_iter()
            .map(|file| EmbeddingModelDownloadFileSummary {
                path: file.path,
                size_bytes: file.size_bytes,
                downloaded_size_bytes: file.downloaded_size_bytes,
                downloaded: file.downloaded,
            })
            .collect(),
    }
}

fn required_text(value: &str, label: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label}不能为空")))
    } else {
        Ok(value.to_string())
    }
}

fn normalize_download_error(err: AppError) -> AppError {
    if is_cancelled_message(&err.to_string()) {
        AppError::Cancelled
    } else {
        err
    }
}

fn is_cancelled_message(message: &str) -> bool {
    message.contains("取消") || message.contains("停止")
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::error::AppError;

    #[test]
    fn blank_model_is_rejected() {
        let err = required_text("  ", "fastembed 模型名").expect_err("blank model should fail");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn download_cancel_message_is_normalized() {
        let err = normalize_download_error(AppError::ConfigError("模型下载已取消".into()));

        assert!(matches!(err, AppError::Cancelled));
    }
}
