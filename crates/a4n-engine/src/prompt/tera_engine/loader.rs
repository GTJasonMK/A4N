use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use a4n_core::error::AppError;
use tera::Tera;

use crate::prompt::engine::PromptEntry;
use crate::prompt::registry::{PromptRegistry, PromptRegistryEntry};

use super::TeraPromptEngine;

struct PlannedPrompt {
    entry: PromptEntry,
    content: String,
}

struct PromptLoadPlan {
    prompts: Vec<PlannedPrompt>,
    skipped: usize,
}

struct RuntimeCommitPlan {
    entries: HashMap<String, PromptEntry>,
    refreshed: usize,
    removed: usize,
}

struct PromptScanContext<'a> {
    root: &'a Path,
    incremental: bool,
    registry: &'a PromptRegistry,
    existing_names: &'a HashSet<String>,
}

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

    pub(super) async fn reload_defaults_from_directory(&self) -> Result<usize, AppError> {
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

    /// 从默认目录规划要加载的提示词，已存在的条目不会被覆盖（保留用户修改）。
    ///
    /// 递归子目录；跳过 `_` 开头的目录（如 `_partials/`，仅供 @include 使用，
    /// 不作为顶层 entry 暴露给用户）。同时跳过根目录下的 `_registry.yaml`。
    ///
    /// `incremental = true` 表示只补全缺失项（用于 load_defaults 重扫）；
    /// `incremental = false` 表示初次加载（仍然 skip 已存在条目，因 from_directory
    /// 从空引擎开始，行为等同）。
    fn plan_scan_dir(
        &self,
        dir: &Path,
        incremental: bool,
        registry: &PromptRegistry,
        existing_names: &HashSet<String>,
    ) -> Result<PromptLoadPlan, AppError> {
        let root = dir.to_path_buf();
        let mut plan = PromptLoadPlan {
            prompts: Vec::new(),
            skipped: 0,
        };
        let mut seen_names = existing_names.clone();
        let context = PromptScanContext {
            root: &root,
            incremental,
            registry,
            existing_names,
        };
        self.plan_scan_dir_recursive(&context, dir, &mut seen_names, &mut plan)?;
        Ok(plan)
    }

    /// 递归实现：root 是 prompts 根目录，cur 是当前正在遍历的子目录
    fn plan_scan_dir_recursive(
        &self,
        context: &PromptScanContext<'_>,
        cur: &Path,
        seen_names: &mut HashSet<String>,
        plan: &mut PromptLoadPlan,
    ) -> Result<(), AppError> {
        let it = fs::read_dir(cur)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("无法读取提示词目录 {cur:?}: {e}")))?;

        for entry in it {
            let entry =
                entry.map_err(|e| AppError::Internal(anyhow::anyhow!("读取目录项失败: {e}")))?;
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            if path.is_dir() {
                // 跳过 _ 开头目录（如 _partials/），它们仅供 @include 使用
                if name_str.starts_with('_') {
                    continue;
                }
                self.plan_scan_dir_recursive(context, &path, seen_names, plan)?;
                continue;
            }

            // 仅处理 .md 且非 _ 开头的文件
            if name_str.starts_with('_') {
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }

            let parsed = crate::prompt::markdown::parse_markdown_file(&path)?;

            if seen_names.contains(&parsed.name) {
                if context.incremental && context.existing_names.contains(&parsed.name) {
                    if context.registry.get(&parsed.name).is_none() {
                        return Err(AppError::ConfigError(format!(
                            "提示词 {} 已不在 registry 中，但文件仍存在：{}",
                            parsed.name,
                            path.display()
                        )));
                    }
                    plan.skipped += 1;
                    continue;
                }
                return Err(AppError::ConfigError(format!(
                    "提示词名称重复：{} ({})",
                    parsed.name,
                    path.display()
                )));
            }

            // 展开 @include 指令（递归 + 循环检测 + 路径越界保护），加载时静态展开。
            let expanded =
                crate::prompt::includes::resolve_includes(&parsed.content, &path, context.root)?;

            let entry = self.build_entry_from_registry(
                context.registry,
                &parsed.name,
                expanded.clone(),
                context.root,
                &path,
            )?;
            let entry_path = entry.path.clone().ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("提示词缺少路径：{}", entry.name))
            })?;

            tracing::info!(
                "加载提示词: {} ({}) [{}]",
                entry.name,
                entry.title,
                entry_path
            );
            seen_names.insert(entry.name.clone());
            plan.prompts.push(PlannedPrompt {
                entry,
                content: expanded,
            });
        }

        Ok(())
    }

    fn commit_load_plan(
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

    /// 从 registry 元数据构造完整 PromptEntry。
    fn build_entry_from_registry(
        &self,
        registry: &PromptRegistry,
        name: &str,
        content: String,
        root: &Path,
        path: &Path,
    ) -> Result<PromptEntry, AppError> {
        let meta = registry
            .get(name)
            .ok_or_else(|| AppError::ConfigError(format!("提示词 {name} 缺少 registry 元数据")))?;

        let rel_path = path.strip_prefix(root).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "提示词路径不在根目录内 {}: {e}",
                path.display()
            ))
        })?;
        let rel_path = rel_path.to_string_lossy().into_owned();
        if rel_path != meta.path {
            return Err(AppError::ConfigError(format!(
                "提示词 {name} 路径与 registry 不一致：文件为 {rel_path}，registry 为 {}",
                meta.path
            )));
        }

        Ok(PromptEntry {
            name: name.to_string(),
            title: meta.title.clone(),
            content,
            description: Some(meta.description.clone()),
            tags: meta.tags.clone(),
            category: meta.category.clone(),
            dependencies: meta.dependencies.clone(),
            is_modified: false,
            version: meta.version.clone(),
            modified_at: chrono::Utc::now().naive_utc(),
            used_by: meta.used_by.clone(),
            path: Some(meta.path.clone()),
        })
    }
}

fn load_validated_registry(dir: &Path) -> Result<PromptRegistry, AppError> {
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

fn runtime_commit_plan_for_registry(
    registry: &PromptRegistry,
    current_entries: HashMap<String, PromptEntry>,
    load_plan: &PromptLoadPlan,
) -> RuntimeCommitPlan {
    let current_count = current_entries.len();
    let mut entries = HashMap::new();
    let mut refreshed = 0usize;

    for (name, mut entry) in current_entries {
        let Some(meta) = registry.get(&name) else {
            continue;
        };
        if refresh_entry_registry_metadata(&mut entry, meta) {
            refreshed += 1;
        }
        entries.insert(name, entry);
    }

    for planned in &load_plan.prompts {
        entries.insert(planned.entry.name.clone(), planned.entry.clone());
    }

    RuntimeCommitPlan {
        removed: current_count.saturating_sub(entries.len() - load_plan.prompts.len()),
        entries,
        refreshed,
    }
}

fn staged_tera_for_entries(entries: &HashMap<String, PromptEntry>) -> Result<Tera, AppError> {
    let mut tera = Tera::default();
    for entry in entries.values() {
        tera.add_raw_template(&entry.name, &entry.content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("模板编译失败 {}: {e}", entry.name)))?;
    }
    Ok(tera)
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
