use std::collections::HashSet;
use std::fs;
use std::path::Path;

use a4n_error::AppError;

use super::contracts::{PlannedPrompt, PromptLoadPlan, PromptScanContext};
use crate::engine::PromptEntry;
use crate::registry::PromptRegistry;
use crate::tera_engine::TeraPromptEngine;

impl TeraPromptEngine {
    /// 从默认目录规划要加载的提示词，已存在的条目不会被覆盖（保留用户修改）。
    ///
    /// 递归子目录；跳过 `_` 开头的目录（如 `_partials/`，仅供 @include 使用，
    /// 不作为顶层 entry 暴露给用户）。同时跳过根目录下的 `_registry.yaml`。
    ///
    /// `incremental = true` 表示只补全缺失项（用于 load_defaults 重扫）；
    /// `incremental = false` 表示初次加载（仍然 skip 已存在条目，因 from_directory
    /// 从空引擎开始，行为等同）。
    pub(super) fn plan_scan_dir(
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
                if name_str.starts_with('_') {
                    continue;
                }
                self.plan_scan_dir_recursive(context, &path, seen_names, plan)?;
                continue;
            }

            if name_str.starts_with('_') {
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "md") {
                continue;
            }

            let parsed = crate::markdown::parse_markdown_file(&path)?;

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

            let expanded = crate::includes::resolve_includes(&parsed.content, &path, context.root)?;
            let entry = self.build_entry_from_registry(
                context.registry,
                &parsed.name,
                expanded.clone(),
                context.root,
                &path,
            )?;
            let entry_path = entry.path.clone();

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
        let rel_path = rel_path
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
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
            description: meta.description.clone(),
            tags: meta.tags.clone(),
            category: meta.category.clone(),
            dependencies: meta.dependencies.clone(),
            is_modified: false,
            version: meta.version.clone(),
            modified_at: chrono::Utc::now().naive_utc(),
            used_by: meta.used_by.clone(),
            path: meta.path.clone(),
        })
    }
}
