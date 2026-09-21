use std::path::{Path, PathBuf};

use a4n_domain::ProviderError as AppError;
use fastembed::{EmbeddingModel, ModelInfo};

const TOKENIZER_FILES: [&str; 4] = [
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
];

pub(super) fn model_cache_path(info: &ModelInfo<EmbeddingModel>, cache_dir: &Path) -> PathBuf {
    cache_dir.join(repo_folder_name(&info.model_code))
}

pub(super) fn repo_folder_name(model_code: &str) -> String {
    format!("models--{}", model_code).replace('/', "--")
}

pub(super) fn snapshot_with_required_files(
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

pub(super) fn required_files(info: &ModelInfo<EmbeddingModel>) -> Vec<String> {
    let mut files = Vec::with_capacity(1 + TOKENIZER_FILES.len() + info.additional_files.len());
    files.push(info.model_file.clone());
    files.extend(TOKENIZER_FILES.iter().map(|file| (*file).to_string()));
    files.extend(info.additional_files.iter().cloned());
    files
}

pub(super) fn dir_size(path: &Path) -> Result<u64, AppError> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }
    Ok(total)
}
