use std::path::{Path, PathBuf};

use super::contract::sync_contract_bound_prompts;
use super::registry::{
    merge_missing_prompt_registry_entries, sync_contract_bound_prompt_registry_entries,
};

pub(in crate::state) fn ensure_portable_prompts_dir(data_dir: &Path) -> Result<PathBuf, String> {
    let portable_dir = data_dir.join("prompts");
    if portable_dir.join("_registry.yaml").is_file() {
        sync_missing_bundled_prompts(data_dir, &portable_dir)
            .map_err(|err| format!("同步随包新增 prompts 失败：{err}"))?;
        tracing::info!("已定位用户 prompts 目录：{portable_dir:?}");
        return Ok(portable_dir);
    }

    for source in bundled_prompts_candidates(data_dir) {
        if source.join("_registry.yaml").is_file() {
            if let Err(err) = copy_dir_missing(&source, &portable_dir) {
                tracing::warn!(
                    source = ?source,
                    dest = ?portable_dir,
                    error = %err,
                    "初始化用户 prompts 目录失败"
                );
                continue;
            }
            tracing::info!(
                source = ?source,
                dest = ?portable_dir,
                "已初始化用户 prompts 目录"
            );
            return Ok(portable_dir);
        }
    }

    Err("未找到 prompts 目录（已在 DATA_DIR/CWD/EXE_DIR/share/a4n 下查找）".to_string())
}

fn sync_missing_bundled_prompts(data_dir: &Path, portable_dir: &Path) -> Result<(), String> {
    for source in bundled_prompts_candidates(data_dir) {
        if source == portable_dir || !source.join("_registry.yaml").is_file() {
            continue;
        }
        copy_dir_missing(&source, portable_dir).map_err(|err| {
            format!(
                "复制缺失提示词文件失败：source={}, dest={}：{err}",
                source.display(),
                portable_dir.display()
            )
        })?;
        merge_missing_prompt_registry_entries(&source, portable_dir)?;
        sync_contract_bound_prompt_registry_entries(&source, portable_dir)?;
        sync_contract_bound_prompts(&source, portable_dir)?;
        tracing::info!(
            source = ?source,
            dest = ?portable_dir,
            "已同步随包 prompts 文件与契约元数据"
        );
        return Ok(());
    }
    Ok(())
}

fn bundled_prompts_candidates(data_dir: &Path) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(portable_home) = data_dir.parent() {
        candidates.push(portable_home.join("prompts"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("prompts"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidates.push(parent.join("prompts"));
        if let Some(grand) = parent.parent() {
            candidates.push(grand.join("prompts"));
            candidates.push(grand.join("share").join("a4n").join("prompts"));
        }
    }
    candidates
}

fn copy_dir_missing(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_missing(&src_path, &dest_path)?;
        } else if file_type.is_file() && !dest_path.exists() {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
