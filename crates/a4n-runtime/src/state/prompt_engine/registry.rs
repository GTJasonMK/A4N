use std::collections::hash_map::Entry;
use std::path::Path;

use super::contract::CONTRACT_BOUND_PROMPT_PATHS;

pub(super) fn merge_missing_prompt_registry_entries(
    source_dir: &Path,
    dest_dir: &Path,
) -> Result<(), String> {
    let source_path = source_dir.join("_registry.yaml");
    let dest_path = dest_dir.join("_registry.yaml");
    let source = read_registry_file(&source_path)?;
    let mut dest = read_registry_file(&dest_path)?;
    let mut changed = false;

    if dest.version.is_none() {
        dest.version = source.version.clone();
        changed = true;
    }
    if dest.last_updated.is_none() {
        dest.last_updated = source.last_updated.clone();
        changed = true;
    }
    for (key, value) in source.categories {
        if let Entry::Vacant(entry) = dest.categories.entry(key) {
            entry.insert(value);
            changed = true;
        }
    }
    for (key, value) in source.prompts {
        if let Entry::Vacant(entry) = dest.prompts.entry(key) {
            entry.insert(value);
            changed = true;
        }
    }

    if changed {
        write_registry_file(&dest_path, &dest)?;
    }
    Ok(())
}

pub(super) fn sync_contract_bound_prompt_registry_entries(
    source_dir: &Path,
    dest_dir: &Path,
) -> Result<(), String> {
    let source_path = source_dir.join("_registry.yaml");
    let dest_path = dest_dir.join("_registry.yaml");
    let source = read_registry_file(&source_path)?;
    let mut dest = read_registry_file(&dest_path)?;
    let mut changed = Vec::new();

    for (name, source_entry) in source.prompts {
        if !CONTRACT_BOUND_PROMPT_PATHS.contains(&source_entry.path.as_str()) {
            continue;
        }
        if dest.prompts.get(&name) == Some(&source_entry) {
            continue;
        }
        dest.prompts.insert(name.clone(), source_entry);
        changed.push(name);
    }

    if changed.is_empty() {
        return Ok(());
    }

    write_registry_file(&dest_path, &dest)?;
    tracing::info!(
        prompts = %changed.join("、"),
        source = ?source_dir,
        dest = ?dest_dir,
        "已同步后端契约绑定 prompts 注册元数据"
    );
    Ok(())
}

fn read_registry_file(path: &Path) -> Result<a4n_prompt::registry::RegistryFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("读取提示词注册表失败：{}：{err}", path.display()))?;
    serde_yaml::from_str(&text)
        .map_err(|err| format!("解析提示词注册表失败：{}：{err}", path.display()))
}

fn write_registry_file(
    path: &Path,
    registry: &a4n_prompt::registry::RegistryFile,
) -> Result<(), String> {
    let text =
        serde_yaml::to_string(registry).map_err(|err| format!("序列化提示词注册表失败：{err}"))?;
    let temp_path = path.with_extension("yaml.tmp");
    std::fs::write(&temp_path, text)
        .map_err(|err| format!("写入临时提示词注册表失败：{}：{err}", temp_path.display()))?;
    std::fs::rename(&temp_path, path).map_err(|err| {
        let primary = format!(
            "替换提示词注册表失败：{} -> {}：{err}",
            temp_path.display(),
            path.display()
        );
        crate::file_utils::error_with_rollback_file_cleanup(
            primary,
            std::slice::from_ref(&temp_path),
            "临时提示词注册表",
        )
    })
}
