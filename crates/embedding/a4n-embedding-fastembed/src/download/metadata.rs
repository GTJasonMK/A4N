use std::path::PathBuf;

use fastembed::{EmbeddingModel, ModelInfo};
use hf_hub::{Cache, Repo, RepoType};

use super::FastEmbedDownloadFile;
use super::cache::cached_file_size;
use super::client::build_hf_api;
use crate::proxy::ScopedHfHubEnv;
use crate::required_files;

pub(super) async fn collect_required_download_files(
    info: &ModelInfo<EmbeddingModel>,
    cache_dir: PathBuf,
) -> Result<Vec<FastEmbedDownloadFile>, String> {
    std::fs::create_dir_all(&cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    let api = {
        let _hf_env = ScopedHfHubEnv::set_for_download(&cache_dir)?;
        build_hf_api(&cache_dir)?
    };
    let repo = api.model(info.model_code.clone());
    let cache_repo = Cache::new(cache_dir.to_path_buf())
        .repo(Repo::new(info.model_code.clone(), RepoType::Model));
    let mut files = Vec::new();
    for path in required_files(info) {
        let url = repo.url(&path);
        let metadata = api
            .metadata(&url)
            .await
            .map_err(|err| format!("读取模型文件大小失败：{path}：{err}"))?;
        let size_bytes = metadata.size() as u64;
        let cached_bytes = cached_file_size(&cache_repo, &path, metadata.etag(), size_bytes)?;
        files.push(FastEmbedDownloadFile {
            url,
            path,
            size_bytes,
            downloaded_size_bytes: cached_bytes,
            etag: metadata.etag().to_string(),
            commit_hash: metadata.commit_hash().to_string(),
        });
    }
    Ok(files)
}
