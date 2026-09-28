use super::paths::path_to_string;
use super::{OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary};
use std::{collections::HashSet, io::ErrorKind, path::Path};

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
        if project_id <= 0
            || name != project_id.to_string()
            || existing_project_ids.contains(&project_id)
        {
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

pub(super) fn confirmed_orphan_project_data_dirs(
    confirmed_project_ids: &[i64],
    current: &OrphanProjectDataDirsSummary,
) -> Result<OrphanProjectDataDirsSummary, String> {
    if confirmed_project_ids.is_empty() {
        return Err("未提供已确认的孤儿项目目录".to_string());
    }

    let mut selected_ids = HashSet::with_capacity(confirmed_project_ids.len());
    for project_id in confirmed_project_ids {
        if *project_id <= 0 || !selected_ids.insert(*project_id) {
            return Err("已确认的项目编号无效".to_string());
        }
    }

    let items: Vec<_> = current
        .items
        .iter()
        .filter(|item| selected_ids.contains(&item.project_id))
        .cloned()
        .collect();
    if items.len() != selected_ids.len() {
        return Err("孤儿目录列表已变化，请重新扫描后再清理".to_string());
    }

    Ok(OrphanProjectDataDirsSummary {
        total_count: items.len(),
        total_size_bytes: items.iter().map(|item| item.size_bytes).sum(),
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
