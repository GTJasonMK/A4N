use super::paths::path_to_string;
use super::{OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary};
use crate::file_utils::path_to_db_string;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs::Metadata, io::ErrorKind, path::Path, time::UNIX_EPOCH};

const MANAGED_MEDIA_DIRS: [&str; 6] = ["images", "videos", "portraits", "audio", "media", "assets"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManagedMediaFile {
    pub project_id: i64,
    pub relative_path: String,
    pub version: String,
}

pub(super) fn managed_media_files(data_dir: &Path) -> Result<Vec<ManagedMediaFile>, String> {
    let projects = match std::fs::read_dir(data_dir.join("projects")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("读取项目媒体目录失败：{error}")),
    };
    let mut files = Vec::new();
    for project in projects {
        let project = project.map_err(|error| format!("读取项目媒体目录项失败：{error}"))?;
        if !project
            .file_type()
            .map_err(|error| format!("读取项目媒体目录类型失败：{error}"))?
            .is_dir()
        {
            continue;
        }
        let Some(project_id) = project
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
        else {
            continue;
        };
        for dir in MANAGED_MEDIA_DIRS {
            collect_managed_media_files(
                data_dir,
                project_id,
                &project.path().join(dir),
                &mut files,
            )?;
        }
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn collect_managed_media_files(
    data_dir: &Path,
    project_id: i64,
    dir: &Path,
    files: &mut Vec<ManagedMediaFile>,
) -> Result<(), String> {
    let metadata = match std::fs::symlink_metadata(dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("读取受管媒体目录元数据失败：{error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|error| format!("读取受管媒体目录失败：{error}"))?
    {
        let entry = entry.map_err(|error| format!("读取受管媒体目录项失败：{error}"))?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("读取受管媒体文件元数据失败：{error}"))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_managed_media_files(data_dir, project_id, &path, files)?;
            continue;
        }
        if !metadata.is_file() || complete_assembly_cache_member(&path) {
            continue;
        }
        let relative = path
            .strip_prefix(data_dir)
            .map_err(|_| format!("受管媒体文件不在数据目录内：{}", path.display()))?;
        let relative_path = path_to_db_string(relative);
        files.push(ManagedMediaFile {
            project_id,
            version: managed_media_file_version(&relative_path, &metadata)?,
            relative_path,
        });
    }
    Ok(())
}

fn complete_assembly_cache_member(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if let Some(output_name) = name.strip_suffix(".cache.json") {
        return output_name.starts_with("assembled_")
            && output_name.ends_with(".mp4")
            && path.with_file_name(output_name).is_file();
    }
    name.starts_with("assembled_")
        && name.ends_with(".mp4")
        && path.with_file_name(format!("{name}.cache.json")).is_file()
}

fn managed_media_file_version(relative_path: &str, metadata: &Metadata) -> Result<String, String> {
    // ponytail: path/size/mtime fingerprint avoids hashing large videos; add content or inode
    // identity only if same-metadata rewrites become a real recovery case.
    let modified = metadata
        .modified()
        .map_err(|error| format!("读取受管媒体文件修改时间失败：{error}"))?
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("受管媒体文件修改时间早于 UNIX_EPOCH：{error}"))?;
    let mut hasher = Sha256::new();
    hasher.update(relative_path.as_bytes());
    hasher.update(metadata.len().to_le_bytes());
    hasher.update(modified.as_nanos().to_le_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn orphan_project_data_dirs(
    data_dir: &Path,
    existing_project_ids: &HashSet<i64>,
) -> Result<OrphanProjectDataDirsSummary, String> {
    let projects_dir = data_dir.join("projects");
    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(OrphanProjectDataDirsSummary {
                items: Vec::new(),
                total_count: 0,
                total_size_bytes: 0,
            });
        }
        Err(error) => return Err(format!("读取项目数据目录失败：{error}")),
    };

    let mut items = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("读取项目数据目录项失败：{error}"))?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        let Ok(project_id) = name.parse::<i64>() else {
            continue;
        };
        if project_id <= 0 || existing_project_ids.contains(&project_id) {
            continue;
        }
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("读取项目数据目录元数据失败：{error}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        items.push(OrphanProjectDataDirSummary {
            project_id,
            path: path_to_string(&path),
            size_bytes: dir_size_without_following_symlinks(&path)?,
        });
    }
    items.sort_by_key(|item| item.project_id);
    let total_count = items.len();
    let total_size_bytes = items.iter().map(|item| item.size_bytes).sum();
    Ok(OrphanProjectDataDirsSummary {
        total_count,
        total_size_bytes,
        items,
    })
}

fn dir_size_without_following_symlinks(path: &Path) -> Result<u64, String> {
    let mut total = 0_u64;
    let entries = std::fs::read_dir(path).map_err(|error| format!("读取目录大小失败：{error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("读取目录大小项失败：{error}"))?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("读取目录大小元数据失败：{error}"))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            total = total
                .checked_add(dir_size_without_following_symlinks(&path)?)
                .ok_or_else(|| "计算目录大小时超出 u64 范围".to_string())?;
        } else {
            total = total
                .checked_add(metadata.len())
                .ok_or_else(|| "计算目录大小时超出 u64 范围".to_string())?;
        }
    }
    Ok(total)
}
