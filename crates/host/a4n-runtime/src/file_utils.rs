#[cfg(test)]
mod tests;

use std::path::Path;

use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_storage::canonicalize_data_file_path;
use sha2::{Digest, Sha256};

pub(crate) use a4n_storage::{inspect_data_file_facts, promote_data_file};

pub(crate) fn inspect_generated_media_file(
    data_dir: &Path,
    relative_path: &str,
    label: &str,
    content_type_from_path: impl FnOnce(&Path) -> Result<&'static str, String>,
) -> Result<GeneratedMediaArtifactFile, String> {
    let path = canonicalize_data_file_path(data_dir, relative_path, label)?;
    let bytes =
        std::fs::read(&path).map_err(|error| format!("读取{}失败：{error}", path.display()))?;
    Ok(GeneratedMediaArtifactFile {
        content_type: content_type_from_path(&path)?.to_string(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        byte_size: bytes.len() as u64,
    })
}
