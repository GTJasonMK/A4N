use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use tera::Tera;

use super::contracts::{PromptLoadPlan, RuntimeCommitPlan};
use crate::engine::PromptEntry;
use crate::registry::PromptRegistry;
use crate::tera_engine::TeraPromptEngine;

impl TeraPromptEngine {
    pub(super) fn commit_load_plan(
        &self,
        registry: PromptRegistry,
        plan: PromptLoadPlan,
        runtime_plan: RuntimeCommitPlan,
        staged_tera: Tera,
    ) {
        let count = plan.prompts.len();
        let skipped = plan.skipped;
        let refreshed = runtime_plan.refreshed;
        let removed = runtime_plan.removed;
        let retained_names = runtime_plan.entries.keys().cloned().collect::<HashSet<_>>();

        {
            let mut originals = self.original_contents.write();
            originals.retain(|name, _| retained_names.contains(name));
            for planned in plan.prompts {
                originals.insert(planned.entry.name.clone(), planned.content);
            }
        }
        self.version_log
            .write()
            .retain(|name, _| retained_names.contains(name));
        self.overrides
            .write()
            .retain(|(_, name), _| retained_names.contains(name));
        *self.entries.write() = runtime_plan.entries;
        *self.tera.write() = staged_tera;
        *self.registry.write() = registry;
        tracing::info!(
            "已载入默认提示词 {count} 个（跳过已存在 {skipped} 个，刷新元数据 {refreshed} 个，移除退役 {removed} 个）"
        );
    }
}

pub(super) fn staged_tera_for_entries(
    entries: &HashMap<String, PromptEntry>,
) -> Result<Tera, AppError> {
    let mut tera = Tera::default();
    for entry in entries.values() {
        tera.add_raw_template(&entry.name, &entry.content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败 {}: {e}", entry.name)))?;
    }
    Ok(tera)
}
