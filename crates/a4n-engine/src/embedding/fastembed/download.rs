use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use fastembed::{EmbeddingModel, ModelInfo};
use hf_hub::{Cache, Repo, RepoType};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use a4n_core::error::AppError;

mod cache;
mod client;
mod metadata;
mod progress;
mod transfer;

use super::proxy::ScopedHfHubEnv;
use super::{build_text_embedding, model_status_from_info, resolve_model_info};
use crate::embedding::provider::DownloadProgress;
use client::{build_download_client, build_hf_api, ensure_not_cancelled};
use metadata::{collect_required_download_files, collect_required_download_files_for_download};
use progress::{AggregatedDownloadProgress, format_bytes, send_download_progress};
use transfer::download_required_file;

pub(super) const DOWNLOAD_CANCELLED_MESSAGE: &str = "模型下载已取消";

#[derive(Debug, Clone)]
pub struct FastEmbedDownloadInfo {
    pub model_name: String,
    pub model_code: String,
    pub total_size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub missing_size_bytes: u64,
    pub files: Vec<FastEmbedDownloadFile>,
}

#[derive(Debug, Clone)]
pub struct FastEmbedDownloadFile {
    pub path: String,
    pub size_bytes: u64,
    pub downloaded_size_bytes: u64,
    pub downloaded: bool,
    pub(super) url: String,
    pub(super) etag: String,
    pub(super) commit_hash: String,
}

pub async fn fastembed_model_download_info(
    model_name: &str,
    cache_dir: PathBuf,
) -> Result<FastEmbedDownloadInfo, AppError> {
    let resolved = resolve_model_info(model_name)?;
    let model_name = resolved.name.clone();
    let model_code = resolved.info.model_code.clone();
    tokio::task::spawn_blocking(move || {
        collect_fastembed_download_info(&model_name, &model_code, &resolved.info, &cache_dir)
    })
    .await
    .map_err(|err| AppError::EmbeddingError(format!("读取 fastembed 下载信息线程失败: {err}")))?
    .map_err(AppError::ConfigError)
}

pub async fn download_fastembed_model(
    model_name: &str,
    cache_dir: PathBuf,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<(), AppError> {
    let resolved = resolve_model_info(model_name)?;
    send_download_progress(progress_tx.as_ref(), 0, 0, "正在读取模型文件大小…");

    let model = resolved.info.model.clone();
    let info_for_download = resolved.info.clone();
    let model_name = resolved.name.clone();
    let model_code = resolved.info.model_code.clone();
    let model_file = resolved.info.model_file.clone();
    let cache_dir_for_error = cache_dir.clone();

    if let Err(err) = prefetch_fastembed_model_files(
        &info_for_download,
        &cache_dir,
        progress_tx.as_ref(),
        cancel_token.as_ref(),
    )
    .await
    {
        return Err(AppError::ConfigError(fastembed_download_error(
            &model_name,
            &model_code,
            &model_file,
            &cache_dir_for_error,
            &err,
        )));
    }
    if let Err(err) = ensure_not_cancelled(cancel_token.as_ref()) {
        return Err(AppError::ConfigError(fastembed_download_error(
            &model_name,
            &model_code,
            &model_file,
            &cache_dir_for_error,
            &err,
        )));
    }

    let build_cache_dir = cache_dir.clone();
    let mut build_handle =
        tokio::task::spawn_blocking(move || build_text_embedding(model, build_cache_dir, false));
    let build_result = match cancel_token.as_ref() {
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => {
                    build_handle.abort();
                    return Err(AppError::ConfigError(fastembed_download_error(
                        &model_name,
                        &model_code,
                        &model_file,
                        &cache_dir_for_error,
                        DOWNLOAD_CANCELLED_MESSAGE,
                    )));
                }
                result = &mut build_handle => result,
            }
        }
        None => build_handle.await,
    }
    .map_err(|err| AppError::EmbeddingError(format!("初始化 fastembed 模型线程失败: {err}")))?;
    if let Err(err) = build_result {
        return Err(AppError::ConfigError(fastembed_download_error(
            &model_name,
            &model_code,
            &model_file,
            &cache_dir_for_error,
            &err,
        )));
    }

    let status = model_status_from_info(resolved.info, &cache_dir_for_error)?;
    if !status.downloaded {
        return Err(AppError::ConfigError(format!(
            "fastembed 模型 {model_name} 下载结束但缓存仍不完整，请删除缓存后重新下载；缓存目录：{}",
            cache_dir_for_error.to_string_lossy()
        )));
    }

    Ok(())
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

fn collect_fastembed_download_info(
    model_name: &str,
    model_code: &str,
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
) -> Result<FastEmbedDownloadInfo, String> {
    std::fs::create_dir_all(cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    let _hf_env = ScopedHfHubEnv::set(cache_dir)?;
    let api = build_hf_api(cache_dir)?;
    let repo = api.model(model_code.to_string());
    let files = collect_required_download_files(&api, &repo, info, cache_dir)?;
    let total_size_bytes = files.iter().map(|file| file.size_bytes).sum::<u64>();
    let downloaded_size_bytes = files
        .iter()
        .map(|file| file.downloaded_size_bytes)
        .sum::<u64>();
    let missing_size_bytes = total_size_bytes.saturating_sub(downloaded_size_bytes);

    Ok(FastEmbedDownloadInfo {
        model_name: model_name.to_string(),
        model_code: model_code.to_string(),
        total_size_bytes,
        downloaded_size_bytes,
        missing_size_bytes,
        files,
    })
}

async fn prefetch_fastembed_model_files(
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
    progress_tx: Option<&UnboundedSender<DownloadProgress>>,
    cancel_token: Option<&CancellationToken>,
) -> Result<(), String> {
    std::fs::create_dir_all(cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    ensure_not_cancelled(cancel_token)?;
    let files = collect_required_download_files_for_download(
        info.clone(),
        cache_dir.to_path_buf(),
        cancel_token,
    )
    .await?;
    ensure_not_cancelled(cancel_token)?;
    let cache_repo = Cache::new(cache_dir.to_path_buf())
        .repo(Repo::new(info.model_code.clone(), RepoType::Model));
    let total_size_bytes = files.iter().map(|file| file.size_bytes).sum::<u64>();
    let cached_size_bytes = files
        .iter()
        .map(|file| file.downloaded_size_bytes)
        .sum::<u64>();

    send_download_progress(
        progress_tx,
        cached_size_bytes,
        total_size_bytes,
        &format!(
            "模型文件总大小 {}，正在准备下载…",
            format_bytes(total_size_bytes)
        ),
    );

    if cached_size_bytes >= total_size_bytes {
        send_download_progress(
            progress_tx,
            total_size_bytes,
            total_size_bytes,
            "模型文件已在本地缓存，正在初始化模型…",
        );
        return Ok(());
    }

    let completed = Arc::new(AtomicU64::new(cached_size_bytes));
    let client = build_download_client(cache_dir)?;
    for file in files.iter().filter(|file| !file.downloaded) {
        ensure_not_cancelled(cancel_token)?;
        let mut progress = AggregatedDownloadProgress::new(
            progress_tx.cloned(),
            completed.clone(),
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
        "模型文件下载完成，正在初始化模型…",
    );
    Ok(())
}
