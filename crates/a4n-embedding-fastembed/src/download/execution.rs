use fastembed::{EmbeddingModel, ModelInfo};
use hf_hub::{Cache, Repo, RepoType};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use a4n_domain::ProviderError as AppError;
use a4n_embedding::provider::DownloadProgress;
use a4n_embedding_contract::LocalEmbeddingModelSummary;

use super::DOWNLOAD_CANCELLED_MESSAGE;
use super::client::{build_download_client, ensure_not_cancelled};
use super::contracts::download_byte_totals;
use super::metadata::collect_required_download_files;
use super::progress::{AggregatedDownloadProgress, format_bytes, send_download_progress};
use super::transfer::download_required_file;
use crate::models::{model_status_from_info, resolve_model_info};
use crate::provider::build_text_embedding;

pub async fn download_fastembed_model(
    model_name: &str,
    cache_dir: PathBuf,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: CancellationToken,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    let resolved = resolve_model_info(model_name)?;
    send_download_progress(progress_tx.as_ref(), 0, 0, "正在读取模型文件大小...");

    let model = resolved.info.model.clone();

    if let Err(err) = prefetch_fastembed_model_files(
        &resolved.info,
        &cache_dir,
        progress_tx.as_ref(),
        &cancel_token,
    )
    .await
    {
        if cancel_token.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        return Err(AppError::Configuration(fastembed_download_error(
            &resolved.name,
            &resolved.info.model_code,
            &resolved.info.model_file,
            &cache_dir,
            &err,
        )));
    }
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled);
    }

    let build_cache_dir = cache_dir.clone();
    let mut build_handle =
        tokio::task::spawn_blocking(move || build_text_embedding(model, build_cache_dir));
    let build_result = tokio::select! {
        _ = cancel_token.cancelled() => {
            build_handle.abort();
            return Err(AppError::Cancelled);
        }
        result = &mut build_handle => result,
    }
    .map_err(|err| AppError::Embedding(format!("初始化 fastembed 模型线程失败: {err}")))?;
    if let Err(err) = build_result {
        return Err(AppError::Configuration(fastembed_download_error(
            &resolved.name,
            &resolved.info.model_code,
            &resolved.info.model_file,
            &cache_dir,
            &err,
        )));
    }

    let status = model_status_from_info(resolved.info, &cache_dir)?;
    if !status.downloaded {
        return Err(AppError::Configuration(format!(
            "fastembed 模型 {} 下载结束但缓存仍不完整，请删除缓存后重新下载；缓存目录：{}",
            resolved.name,
            cache_dir.to_string_lossy()
        )));
    }

    Ok(status)
}

fn fastembed_download_error(
    model_name: &str,
    model_code: &str,
    model_file: &str,
    cache_dir: &Path,
    err: &str,
) -> String {
    format!(
        "fastembed 模型 {model_name} 下载失败：无法获取模型文件 {model_file}（仓库 {model_code}）。\
请确认当前网络能访问 HuggingFace，或设置 HF_ENDPOINT 为可用镜像后重启应用再下载；缓存目录：{}；底层错误：{err}",
        cache_dir.to_string_lossy()
    )
}

async fn prefetch_fastembed_model_files(
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
    progress_tx: Option<&UnboundedSender<DownloadProgress>>,
    cancel_token: &CancellationToken,
) -> Result<(), String> {
    std::fs::create_dir_all(cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    ensure_not_cancelled(cancel_token)?;
    let files = tokio::select! {
        _ = cancel_token.cancelled() => return Err(DOWNLOAD_CANCELLED_MESSAGE.to_string()),
        files = collect_required_download_files(info, cache_dir.to_path_buf()) => files,
    }?;
    ensure_not_cancelled(cancel_token)?;
    let cache_repo = Cache::new(cache_dir.to_path_buf())
        .repo(Repo::new(info.model_code.clone(), RepoType::Model));
    let (total_size_bytes, cached_size_bytes) = download_byte_totals(&files)?;
    if total_size_bytes == 0 {
        return Err(format!(
            "fastembed 模型下载信息未包含有效文件大小：{}",
            info.model_code
        ));
    }

    send_download_progress(
        progress_tx,
        cached_size_bytes,
        total_size_bytes,
        &format!(
            "模型文件总大小 {}，正在准备下载...",
            format_bytes(total_size_bytes)
        ),
    );

    if cached_size_bytes == total_size_bytes {
        send_download_progress(
            progress_tx,
            total_size_bytes,
            total_size_bytes,
            "模型文件已在本地缓存，正在初始化模型...",
        );
        return Ok(());
    }

    let mut completed = cached_size_bytes;
    let client = build_download_client(cache_dir)?;
    for file in files
        .iter()
        .filter(|file| file.downloaded_size_bytes < file.size_bytes)
    {
        ensure_not_cancelled(cancel_token)?;
        let mut progress = AggregatedDownloadProgress::new(
            progress_tx.cloned(),
            &mut completed,
            total_size_bytes,
            file.path.clone(),
        );
        download_required_file(&client, &cache_repo, file, &mut progress, cancel_token)
            .await
            .map_err(|err| format!("下载模型文件 {} 失败：{err}", file.path))?;
    }

    send_download_progress(
        progress_tx,
        total_size_bytes,
        total_size_bytes,
        "模型文件下载完成，正在初始化模型...",
    );
    Ok(())
}
