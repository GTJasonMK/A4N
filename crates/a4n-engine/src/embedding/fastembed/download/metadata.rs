use std::path::{Path, PathBuf};

use fastembed::{EmbeddingModel, ModelInfo};
use hf_hub::api::sync::{Api, ApiRepo};
use hf_hub::{Cache, Repo, RepoType};
use tokio_util::sync::CancellationToken;

use super::cache::cached_file_size;
use super::client::build_hf_api;
use super::{DOWNLOAD_CANCELLED_MESSAGE, FastEmbedDownloadFile};
use crate::embedding::fastembed::proxy::ScopedHfHubEnv;
use crate::embedding::fastembed::required_files;

pub(super) async fn collect_required_download_files_for_download(
    info: ModelInfo<EmbeddingModel>,
    cache_dir: PathBuf,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<FastEmbedDownloadFile>, String> {
    let handle = tokio::task::spawn_blocking(move || {
        let _hf_env = ScopedHfHubEnv::set(&cache_dir)?;
        let api = build_hf_api(&cache_dir)?;
        let repo = api.model(info.model_code.clone());
        collect_required_download_files(&api, &repo, &info, &cache_dir)
    });

    let result = match cancel_token {
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => return Err(DOWNLOAD_CANCELLED_MESSAGE.to_string()),
                result = handle => result,
            }
        }
        None => handle.await,
    };
    result.map_err(|err| format!("读取模型文件大小线程失败：{err}"))?
}

pub(super) fn collect_required_download_files(
    api: &Api,
    repo: &ApiRepo,
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
) -> Result<Vec<FastEmbedDownloadFile>, String> {
    let cache_repo = Cache::new(cache_dir.to_path_buf())
        .repo(Repo::new(info.model_code.clone(), RepoType::Model));
    required_files(info)
        .into_iter()
        .map(|path| {
            let metadata = api
                .metadata(&repo.url(&path))
                .map_err(|err| format!("读取模型文件大小失败：{}：{err}", path))?;
            let size_bytes = metadata.size() as u64;
            let cached_bytes = cached_file_size(&cache_repo, &path, metadata.etag(), size_bytes);
            Ok(FastEmbedDownloadFile {
                url: repo.url(&path),
                path,
                size_bytes,
                downloaded_size_bytes: cached_bytes,
                downloaded: cached_bytes >= size_bytes,
                etag: metadata.etag().to_string(),
                commit_hash: metadata.commit_hash().to_string(),
            })
        })
        .collect()
}
