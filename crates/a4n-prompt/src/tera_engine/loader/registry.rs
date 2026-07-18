use std::collections::HashMap;
use std::path::Path;

use a4n_core::error::AppError;

use super::contracts::{PromptLoadPlan, RuntimeCommitPlan};
use crate::engine::PromptEntry;
use crate::registry::{PromptRegistry, PromptRegistryEntry};

pub(super) fn load_validated_registry(dir: &Path) -> Result<PromptRegistry, AppError> {
    let registry = PromptRegistry::load_from_dir(dir)?;
    let registry_errors = registry.validate(dir);
    if !registry_errors.is_empty() {
        return Err(AppError::ConfigError(format!(
            "提示词注册表校验失败：{}",
            registry_errors.join("；")
        )));
    }
    Ok(registry)
}

pub(super) fn runtime_commit_plan_for_registry(
    registry: &PromptRegistry,
    current_entries: HashMap<String, PromptEntry>,
    load_plan: &PromptLoadPlan,
) -> RuntimeCommitPlan {
    let current_count = current_entries.len();
    let mut entries = HashMap::new();
    let mut retained_current_count = 0usize;
    let mut refreshed = 0usize;

    for (name, mut entry) in current_entries {
        let Some(meta) = registry.get(&name) else {
            continue;
        };
        if refresh_entry_registry_metadata(&mut entry, meta) {
            refreshed += 1;
        }
        entries.insert(name, entry);
        retained_current_count += 1;
    }

    for planned in &load_plan.prompts {
        entries.insert(planned.entry.name.clone(), planned.entry.clone());
    }

    RuntimeCommitPlan {
        removed: current_count - retained_current_count,
        entries,
        refreshed,
    }
}

fn refresh_entry_registry_metadata(entry: &mut PromptEntry, meta: &PromptRegistryEntry) -> bool {
    let mut changed = false;

    changed |= replace_if_changed(&mut entry.title, meta.title.clone());
    changed |= replace_if_changed(&mut entry.description, Some(meta.description.clone()));
    changed |= replace_if_changed(&mut entry.tags, meta.tags.clone());
    changed |= replace_if_changed(&mut entry.category, meta.category.clone());
    changed |= replace_if_changed(&mut entry.dependencies, meta.dependencies.clone());
    changed |= replace_if_changed(&mut entry.used_by, meta.used_by.clone());
    changed |= replace_if_changed(&mut entry.path, Some(meta.path.clone()));

    changed
}

fn replace_if_changed<T: PartialEq>(target: &mut T, next: T) -> bool {
    if *target == next {
        return false;
    }
    *target = next;
    true
}
