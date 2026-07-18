use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use fastembed::{EmbeddingModel, ModelInfo};
use hf_hub::{Cache, Repo, RepoType};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use a4n_core::error::AppError;
use a4n_embedding::provider::DownloadProgress;

use super::client::{build_download_client, ensure_not_cancelled};
use super::metadata::collect_required_download_files_for_download;
use super::progress::{AggregatedDownloadProgress, format_bytes, send_download_progress};
use super::transfer::download_required_file;
use crate::models::{model_status_from_info, resolve_model_info};
use crate::provider::build_text_embedding;

pub async fn download_fastembed_model(
    model_name: &str,
    cache_dir: PathBuf,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<(), AppError> {
    let resolved = resolve_model_info(model_name)?;
    send_download_progress(progress_tx.as_ref(), 0, 0, "正在读取模型文件大小...");

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
        if cancel_token
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        return Err(AppError::ConfigError(fastembed_download_error(
            &model_name,
            &model_code,
            &model_file,
            &cache_dir_for_error,
            &err,
        )));
    }
    if cancel_token
        .as_ref()
        .is_some_and(CancellationToken::is_cancelled)
    {
        return Err(AppError::Cancelled);
    }

    let build_cache_dir = cache_dir.clone();
    let mut build_handle =
        tokio::task::spawn_blocking(move || build_text_embedding(model, build_cache_dir, false));
    let build_result = match cancel_token.as_ref() {
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => {
                    build_handle.abort();
                    return Err(AppError::Cancelled);
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
            "模型文件总大小 {}，正在准备下载...",
            format_bytes(total_size_bytes)
        ),
    );

    if cached_size_bytes >= total_size_bytes {
        send_download_progress(
            progress_tx,
            total_size_bytes,
            total_size_bytes,
            "模型文件已在本地缓存，正在初始化模型...",
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
        "模型文件下载完成，正在初始化模型...",
    );
    Ok(())
}
