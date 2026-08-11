mod commit;
mod contracts;
mod registry;
mod scan;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use a4n_core::error::AppError;

use self::commit::staged_tera_for_entries;
use self::registry::{load_validated_registry, runtime_commit_plan_for_registry};
use super::TeraPromptEngine;

impl TeraPromptEngine {
    /// 从 Markdown 文件目录加载全部提示词
    ///
    /// 递归扫描目录及子目录下所有 `.md` 文件（跳过 `_` 开头的目录，如 `_partials/`），
    /// 解析 frontmatter 和模板内容；同时加载并校验 `_registry.yaml` 中央元数据。
    /// 把目录路径保存到 `defaults_dir`，供后续 `load_defaults` 增量重扫使用。
    pub fn from_directory(dir: &Path) -> Result<Self, AppError> {
        let engine = Self::new();
        *engine.defaults_dir.write() = Some(dir.to_path_buf());
        let reg = load_validated_registry(dir)?;
        let existing_names = HashSet::new();
        let plan =
            engine.plan_scan_dir(dir, /* incremental = */ false, &reg, &existing_names)?;
        let runtime_plan = runtime_commit_plan_for_registry(&reg, HashMap::new(), &plan);
        let staged_tera = staged_tera_for_entries(&runtime_plan.entries)?;
        engine.commit_load_plan(reg, plan, runtime_plan, staged_tera);
        Ok(engine)
    }

    pub async fn load_defaults(&self) -> Result<usize, AppError> {
        let _update_guard = self.update_lock.lock().await;
        self.reload_defaults_from_directory_locked()
    }

    fn reload_defaults_from_directory_locked(&self) -> Result<usize, AppError> {
        let dir = self.defaults_dir.read().clone().ok_or_else(|| {
            AppError::ConfigError("未设置 prompts 目录，无法载入默认提示词".to_string())
        })?;
        let reg = load_validated_registry(&dir)?;
        let current_entries = self.entries.read().clone();
        let existing_names = current_entries.keys().cloned().collect::<HashSet<_>>();
        let plan =
            self.plan_scan_dir(&dir, /* incremental = */ true, &reg, &existing_names)?;
        let runtime_plan = runtime_commit_plan_for_registry(&reg, current_entries, &plan);
        let staged_tera = staged_tera_for_entries(&runtime_plan.entries)?;
        let count = plan.prompts.len();
        self.commit_load_plan(reg, plan, runtime_plan, staged_tera);
        Ok(count)
    }
}
