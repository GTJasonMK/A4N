use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use a4n_core::error::AppError;
use fastembed::{EmbeddingModel, ModelInfo, TextEmbedding};

use crate::cache::{dir_size, model_cache_path, repo_folder_name, snapshot_with_required_files};

/// 设置页默认推荐的本地模型：中文检索质量和体积之间比较均衡。
pub const DEFAULT_FASTEMBED_MODEL: &str = "BGESmallZHV15";

#[derive(Debug, Clone, serde::Serialize)]
pub struct FastEmbedLocalModel {
    pub name: String,
    pub model_code: String,
    pub description: String,
    pub dimension: usize,
    pub downloaded: bool,
    pub size_bytes: u64,
}

#[derive(Debug)]
pub(super) struct ResolvedModel {
    pub(super) name: String,
    pub(super) info: ModelInfo<EmbeddingModel>,
}

pub fn list_fastembed_local_models(cache_dir: &Path) -> Result<Vec<FastEmbedLocalModel>, AppError> {
    let mut models = Vec::new();
    for info in TextEmbedding::list_supported_models() {
        models.push(model_status_from_info(info, cache_dir)?);
    }
    models.sort_by(|left, right| {
        right
            .downloaded
            .cmp(&left.downloaded)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(models)
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

pub fn remove_incomplete_fastembed_model_caches(cache_dir: &Path) -> Result<Vec<String>, AppError> {
    let mut repositories = BTreeMap::<PathBuf, Vec<ModelInfo<EmbeddingModel>>>::new();
    for info in TextEmbedding::list_supported_models() {
        repositories
            .entry(model_cache_path(&info, cache_dir))
            .or_default()
            .push(info);
    }

    let mut removed = Vec::new();
    for (repo_path, variants) in repositories {
        if !repo_path.exists() {
            continue;
        }
        let mut complete = false;
        for info in &variants {
            if snapshot_with_required_files(info, &repo_path)?.is_some() {
                complete = true;
                break;
            }
        }
        if complete {
            continue;
        }

        remove_path(&repo_path)?;
        let repo_name = repo_folder_name(&variants[0].model_code);
        let lock_path = cache_dir.join(".locks").join(&repo_name);
        if lock_path.exists() {
            remove_path(&lock_path)?;
        }
        removed.push(repo_name);
    }
    Ok(removed)
}

fn remove_path(path: &Path) -> Result<(), AppError> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub(super) fn resolve_model_info(model_name: &str) -> Result<ResolvedModel, AppError> {
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

pub(super) fn model_status_from_info(
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
        size_bytes,
    })
}

pub(super) fn model_cache_status(
    model: &EmbeddingModel,
    cache_dir: &Path,
) -> Result<FastEmbedLocalModel, AppError> {
    let info = TextEmbedding::get_model_info(model)
        .map_err(|err| AppError::ConfigError(err.to_string()))?
        .clone();
    model_status_from_info(info, cache_dir)
}
