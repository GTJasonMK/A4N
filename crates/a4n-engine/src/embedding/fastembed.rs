//! fastembed 本地嵌入模型后端。
//!
//! 这里刻意把“下载模型”和“使用模型”拆开：只有显式下载 API 会触发
//! HuggingFace 拉取；RAG 入库/检索时若模型未缓存完整，会直接失败并提示先下载。

mod download;
mod proxy;

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use fastembed::{EmbeddingModel, ModelInfo, TextEmbedding, TextInitOptions};
use tokio::sync::{Mutex, mpsc::UnboundedSender};

use a4n_core::error::AppError;

pub use self::download::{
    FastEmbedDownloadFile, FastEmbedDownloadInfo, download_fastembed_model,
    fastembed_model_download_info,
};
use self::proxy::ScopedHfHubEnv;
#[cfg(test)]
use self::proxy::proxy_url;
#[cfg(test)]
use self::proxy::{HF_HOME_OVERRIDE_LOCK, SystemProxy, merge_download_proxy, proxy_to_env_vars};
#[cfg(all(test, target_os = "linux"))]
use self::proxy::{parse_gsettings_array, parse_gsettings_string};
use super::provider::{DownloadProgress, EmbeddingBackend, EmbeddingProvider};

const TOKENIZER_FILES: [&str; 4] = [
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];

/// 设置页默认推荐的本地模型：中文检索质量和体积之间比较均衡。
pub const DEFAULT_FASTEMBED_MODEL: &str = "BGESmallZHV15";

#[derive(Debug, Clone)]
pub struct FastEmbedLocalModel {
    pub name: String,
    pub model_code: String,
    pub description: String,
    pub dimension: usize,
    pub downloaded: bool,
    pub cache_path: PathBuf,
    pub size_bytes: u64,
}

pub struct FastEmbedProvider {
    model_name: String,
    model: EmbeddingModel,
    dimension: usize,
    batch_size: usize,
    cache_dir: PathBuf,
    inner: Arc<Mutex<Option<TextEmbedding>>>,
}

impl FastEmbedProvider {
    pub fn try_new(
        model_name: impl AsRef<str>,
        batch_size: usize,
        cache_dir: PathBuf,
    ) -> Result<Self, AppError> {
        let resolved = resolve_model_info(model_name.as_ref())?;
        if batch_size == 0 {
            return Err(AppError::ConfigError(
                "fastembed 嵌入批量大小必须为正整数".to_string(),
            ));
        }

        Ok(Self {
            model_name: resolved.name,
            model: resolved.info.model.clone(),
            dimension: resolved.info.dim,
            batch_size,
            cache_dir,
            inner: Arc::new(Mutex::new(None)),
        })
    }

    fn ensure_downloaded(&self) -> Result<(), AppError> {
        let status = model_cache_status(&self.model, &self.cache_dir)?;
        if status.downloaded {
            return Ok(());
        }
        Err(AppError::ConfigError(format!(
            "本地嵌入模型 {} 尚未下载，请先在设置页下载后再使用",
            self.model_name
        )))
    }

    async fn take_or_init_model(&self) -> Result<TextEmbedding, AppError> {
        let mut guard = self.inner.lock().await;
        if let Some(model) = guard.take() {
            return Ok(model);
        }

        let model = self.model.clone();
        let cache_dir = self.cache_dir.clone();
        drop(guard);

        tokio::task::spawn_blocking(move || build_text_embedding(model, cache_dir, false))
            .await
            .map_err(|err| AppError::EmbeddingError(format!("初始化 fastembed 线程失败: {err}")))?
            .map_err(|err| {
                AppError::EmbeddingError(format!(
                    "初始化 fastembed 模型失败：本地模型缓存可能不完整，请在设置页删除缓存后重新下载；底层错误：{err}"
                ))
            })
    }

    async fn put_model(&self, model: TextEmbedding) {
        let mut guard = self.inner.lock().await;
        *guard = Some(model);
    }
}

#[async_trait]
impl EmbeddingProvider for FastEmbedProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        self.ensure_downloaded()?;

        let mut model = self.take_or_init_model().await?;
        let batch_size = self.batch_size;
        let (model, embeddings) = tokio::task::spawn_blocking(move || {
            let embeddings = model
                .embed(texts, Some(batch_size))
                .map_err(|err| AppError::EmbeddingError(format!("fastembed 嵌入失败: {err}")));
            (model, embeddings)
        })
        .await
        .map_err(|err| AppError::EmbeddingError(format!("执行 fastembed 线程失败: {err}")))?;
        self.put_model(model).await;
        embeddings
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::FastEmbed
    }

    fn batch_size(&self) -> usize {
        self.batch_size
    }

    async fn download_model(
        &self,
        progress_tx: UnboundedSender<DownloadProgress>,
    ) -> Result<(), AppError> {
        download_fastembed_model(
            &self.model_name,
            self.cache_dir.clone(),
            Some(progress_tx),
            None,
        )
        .await
    }
}

#[derive(Debug)]
struct ResolvedModel {
    name: String,
    info: ModelInfo<EmbeddingModel>,
}

pub fn list_fastembed_local_models(cache_dir: &Path) -> Result<Vec<FastEmbedLocalModel>, AppError> {
    let mut models = Vec::new();
    for info in TextEmbedding::list_supported_models() {
        models.push(model_status_from_info(info, cache_dir)?);
    }
    models.sort_by(|a, b| {
        b.downloaded
            .cmp(&a.downloaded)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(models)
}

pub fn fastembed_model_dimension(model_name: &str) -> Result<usize, AppError> {
    Ok(resolve_model_info(model_name)?.info.dim)
}

pub fn delete_fastembed_model(model_name: &str, cache_dir: &Path) -> Result<(), AppError> {
    let resolved = resolve_model_info(model_name)?;
    let repo_path = model_cache_path(&resolved.info, cache_dir);
    if !repo_path.exists() {
        return Err(AppError::NotFound(format!(
            "本地嵌入模型不存在: {}",
            resolved.name
        )));
    }

    std::fs::remove_dir_all(&repo_path)?;

    let locks_path = cache_dir
        .join(".locks")
        .join(repo_folder_name(&resolved.info.model_code));
    if locks_path.exists() {
        std::fs::remove_dir_all(locks_path)?;
    }
    Ok(())
}

fn build_text_embedding(
    model: EmbeddingModel,
    cache_dir: PathBuf,
    show_download_progress: bool,
) -> Result<TextEmbedding, String> {
    std::fs::create_dir_all(&cache_dir).map_err(|err| format!("创建模型缓存目录失败：{err}"))?;
    let _hf_env = ScopedHfHubEnv::set(&cache_dir)?;
    build_text_embedding_inner(model, cache_dir, show_download_progress)
}

fn build_text_embedding_inner(
    model: EmbeddingModel,
    cache_dir: PathBuf,
    show_download_progress: bool,
) -> Result<TextEmbedding, String> {
    TextEmbedding::try_new(
        TextInitOptions::new(model)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(show_download_progress),
    )
    .map_err(|err| format_error_chain(&err))
}

fn format_error_chain(err: &anyhow::Error) -> String {
    err.chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("；")
}

fn resolve_model_info(model_name: &str) -> Result<ResolvedModel, AppError> {
    let model_name = model_name.trim();
    if model_name.is_empty() {
        return Err(AppError::InvalidArgument(
            "fastembed 模型名不能为空".to_string(),
        ));
    }

    if let Ok(model) = EmbeddingModel::from_str(model_name) {
        let info = TextEmbedding::get_model_info(&model)
            .map_err(|err| AppError::ConfigError(err.to_string()))?
            .clone();
        return Ok(ResolvedModel {
            name: model.to_string(),
            info,
        });
    }

    let matches: Vec<_> = TextEmbedding::list_supported_models()
        .into_iter()
        .filter(|info| info.model_code.eq_ignore_ascii_case(model_name))
        .collect();

    match matches.len() {
        0 => Err(AppError::InvalidArgument(format!(
            "不支持的 fastembed 模型: {model_name}"
        ))),
        1 => {
            let info = matches.into_iter().next().expect("checked len");
            Ok(ResolvedModel {
                name: info.model.to_string(),
                info,
            })
        }
        _ => {
            let variants = matches
                .iter()
                .map(|info| info.model.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(AppError::InvalidArgument(format!(
                "模型代码 {model_name} 对应多个 fastembed 变体，请改用具体模型名: {variants}"
            )))
        }
    }
}

fn model_status_from_info(
    info: ModelInfo<EmbeddingModel>,
    cache_dir: &Path,
) -> Result<FastEmbedLocalModel, AppError> {
    let cache_path = model_cache_path(&info, cache_dir);
    let downloaded = snapshot_with_required_files(&info, &cache_path)?.is_some();
    let size_bytes = if cache_path.exists() {
        dir_size(&cache_path)?
    } else {
        0
    };

    Ok(FastEmbedLocalModel {
        name: info.model.to_string(),
        model_code: info.model_code,
        description: info.description,
        dimension: info.dim,
        downloaded,
        cache_path,
        size_bytes,
    })
}

fn model_cache_status(
    model: &EmbeddingModel,
    cache_dir: &Path,
) -> Result<FastEmbedLocalModel, AppError> {
    let info = TextEmbedding::get_model_info(model)
        .map_err(|err| AppError::ConfigError(err.to_string()))?
        .clone();
    model_status_from_info(info, cache_dir)
}

fn model_cache_path(info: &ModelInfo<EmbeddingModel>, cache_dir: &Path) -> PathBuf {
    cache_dir.join(repo_folder_name(&info.model_code))
}

fn repo_folder_name(model_code: &str) -> String {
    format!("models--{}", model_code).replace('/', "--")
}

fn snapshot_with_required_files(
    info: &ModelInfo<EmbeddingModel>,
    repo_path: &Path,
) -> Result<Option<PathBuf>, AppError> {
    if !repo_path.is_dir() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    let ref_path = repo_path.join("refs").join("main");
    if ref_path.is_file() {
        let commit = std::fs::read_to_string(ref_path)?.trim().to_string();
        if !commit.is_empty() {
            candidates.push(repo_path.join("snapshots").join(commit));
        }
    }

    let snapshots_dir = repo_path.join("snapshots");
    if snapshots_dir.is_dir() {
        for entry in std::fs::read_dir(snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && !candidates.iter().any(|candidate| candidate == &path) {
                candidates.push(path);
            }
        }
    }

    let required_files = required_files(info);
    Ok(candidates.into_iter().find(|snapshot| {
        required_files
            .iter()
            .all(|file| snapshot.join(file).is_file())
    }))
}

fn required_files(info: &ModelInfo<EmbeddingModel>) -> Vec<String> {
    let mut files = Vec::with_capacity(1 + TOKENIZER_FILES.len() + info.additional_files.len());
    files.push(info.model_file.clone());
    files.extend(TOKENIZER_FILES.iter().map(|file| (*file).to_string()));
    files.extend(info.additional_files.iter().cloned());
    files
}

fn dir_size(path: &Path) -> Result<u64, AppError> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_model_by_enum_name() {
        let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
        assert_eq!(resolved.name, DEFAULT_FASTEMBED_MODEL);
        assert_eq!(resolved.info.dim, 512);
    }

    #[test]
    fn rejects_ambiguous_model_code() {
        let err = resolve_model_info("Xenova/all-MiniLM-L12-v2").unwrap_err();
        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("多个 fastembed 变体"));
    }

    #[test]
    fn builds_proxy_urls_only_when_host_and_port_are_present() {
        assert_eq!(
            proxy_url("http", "127.0.0.1", 7890),
            Some("http://127.0.0.1:7890".to_string())
        );
        assert_eq!(proxy_url("http", "", 7890), None);
        assert_eq!(proxy_url("http", "127.0.0.1", 0), None);
    }

    #[test]
    fn merge_download_proxy_prefers_system_http_proxy_over_explicit_socks_all_proxy() {
        let proxy = merge_download_proxy(
            Some(SystemProxy {
                all_proxy: Some("socks5://127.0.0.1:7897".to_string()),
                ..SystemProxy::default()
            }),
            Some(SystemProxy {
                http_proxy: Some("http://127.0.0.1:7897".to_string()),
                https_proxy: Some("http://127.0.0.1:7897".to_string()),
                all_proxy: Some("socks5h://127.0.0.1:7897".to_string()),
                ..SystemProxy::default()
            }),
        )
        .unwrap();

        assert_eq!(proxy.http_proxy.as_deref(), Some("http://127.0.0.1:7897"));
        assert_eq!(proxy.https_proxy.as_deref(), Some("http://127.0.0.1:7897"));
        assert_eq!(proxy.all_proxy.as_deref(), Some("http://127.0.0.1:7897"));
    }

    #[test]
    fn proxy_env_vars_overwrite_all_proxy_with_https_proxy() {
        let _guard = HF_HOME_OVERRIDE_LOCK.lock().unwrap();
        let _vars = proxy_to_env_vars(SystemProxy {
            http_proxy: Some("http://127.0.0.1:7897".to_string()),
            https_proxy: Some("http://127.0.0.1:7897".to_string()),
            all_proxy: Some("socks5://127.0.0.1:7897".to_string()),
            ..SystemProxy::default()
        });

        assert_eq!(
            std::env::var("HTTP_PROXY").unwrap(),
            "http://127.0.0.1:7897"
        );
        assert_eq!(
            std::env::var("HTTPS_PROXY").unwrap(),
            "http://127.0.0.1:7897"
        );
        assert_eq!(std::env::var("ALL_PROXY").unwrap(), "http://127.0.0.1:7897");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_gsettings_proxy_values() {
        assert_eq!(parse_gsettings_string("'manual'"), "manual");
        assert_eq!(
            parse_gsettings_array("['localhost', '127.0.0.1', '*.local']"),
            vec!["localhost", "127.0.0.1", "*.local"]
        );
    }

    #[test]
    fn reports_downloaded_when_required_snapshot_files_exist() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
        let repo_path = model_cache_path(&resolved.info, dir.path());
        let snapshot = repo_path.join("snapshots").join("abc123");
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::create_dir_all(repo_path.join("refs")).unwrap();
        std::fs::write(repo_path.join("refs").join("main"), "abc123").unwrap();

        for file in required_files(&resolved.info) {
            let path = snapshot.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }

        let status = model_status_from_info(resolved.info, dir.path()).unwrap();
        assert!(status.downloaded);
        assert!(status.size_bytes > 0);
    }

    #[test]
    fn reports_missing_when_snapshot_is_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
        let repo_path = model_cache_path(&resolved.info, dir.path());
        let snapshot = repo_path.join("snapshots").join("abc123");
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::create_dir_all(repo_path.join("refs")).unwrap();
        std::fs::write(repo_path.join("refs").join("main"), "abc123").unwrap();
        let model_path = snapshot.join(&resolved.info.model_file);
        std::fs::create_dir_all(model_path.parent().unwrap()).unwrap();
        std::fs::write(model_path, "x").unwrap();

        let status = model_status_from_info(resolved.info, dir.path()).unwrap();
        assert!(!status.downloaded);
    }
}
