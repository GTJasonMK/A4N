use std::{
    fs::File,
    path::{Path, PathBuf},
};

use a4n_domain::media::GeneratedMediaArtifactFile;
use sha2::{Digest, Sha256};

use a4n_storage::{canonicalize_data_file_path, deletable_data_file_path};

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

pub(crate) fn inspect_data_file_facts(
    data_dir: &Path,
    relative_path: &str,
    label: &str,
) -> Result<Option<(PathBuf, i64, String)>, String> {
    let Some(path) = deletable_data_file_path(data_dir, relative_path, label)? else {
        return Ok(None);
    };
    let bytes =
        std::fs::read(&path).map_err(|error| format!("读取{}失败：{error}", path.display()))?;
    let file_size =
        i64::try_from(bytes.len()).map_err(|_| format!("{label}过大，无法记录文件大小"))?;
    Ok(Some((
        path,
        file_size,
        format!("{:x}", Sha256::digest(bytes)),
    )))
}

/// 在同一 data_dir 内提升文件。rename 不承诺目录项断电持久化，发布意图负责重启后对账。
pub(crate) fn promote_data_file(
    data_dir: &Path,
    staging_path: &str,
    final_path: &str,
    label: &str,
) -> Result<(), String> {
    let source = deletable_data_file_path(data_dir, staging_path, label)?
        .ok_or_else(|| format!("{label}暂存文件不存在"))?;
    if deletable_data_file_path(data_dir, final_path, label)?.is_some() {
        return Err(format!("{label}最终文件已存在"));
    }
    let canonical_data_dir = data_dir
        .canonicalize()
        .map_err(|error| format!("无法解析数据目录：{error}"))?;
    let target = canonical_data_dir.join(Path::new(final_path.trim()));
    std::fs::rename(&source, &target).map_err(|error| format!("{label}原子提升失败：{error}"))?;
    File::open(&target)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("{label}提升后同步失败：{error}"))
}
