use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use parking_lot::RwLock;
use tera::{Context, Tera};

use a4n_core::error::AppError;

use crate::engine::{DependencyGraph, GraphEdge, GraphNode, PromptEntry, PromptVersion};

use super::state::TeraPromptEngine;
use super::variables::missing_template_variables;

impl TeraPromptEngine {
    pub(super) fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let (content, is_override) = self.resolve_content(template_name, project_id)?;
        let mut context = Context::new();
        for (key, value) in variables {
            context.insert(key, value);
        }
        if is_override {
            let mut tera = Tera::default();
            tera.add_raw_template(template_name, &content)
                .map_err(|error| AppError::Internal(anyhow::anyhow!("模板编译失败: {error}")))?;
            tera.render(template_name, &context)
                .map_err(|error| AppError::Internal(anyhow::anyhow!("模板渲染失败: {error}")))
        } else {
            self.tera
                .read()
                .render(template_name, &context)
                .map_err(|error| AppError::Internal(anyhow::anyhow!("模板渲染失败: {error}")))
        }
    }

    pub(super) fn get_prompt(&self, name: &str) -> Result<PromptEntry, AppError> {
        self.entries
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| Self::prompt_not_found(name))
    }

    pub(super) fn all_prompts(&self) -> Vec<PromptEntry> {
        self.entries.read().values().cloned().collect()
    }

    pub(super) fn prompt_version_history(
        &self,
        name: &str,
    ) -> Result<Vec<PromptVersion>, AppError> {
        let entries = self.entries.read();
        let entry = entries
            .get(name)
            .ok_or_else(|| Self::prompt_not_found(name))?;
        let mut history = self
            .version_log
            .read()
            .get(name)
            .cloned()
            .unwrap_or_default();
        history.push(PromptVersion {
            version: entry.version.clone(),
            timestamp: entry.modified_at,
            author: "current".into(),
            message: if entry.is_modified {
                "当前修改版本".into()
            } else {
                "原始版本".into()
            },
            content_snapshot: Some(entry.content.clone()),
        });
        Ok(history)
    }

    pub(super) fn prompt_at_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<PromptEntry, AppError> {
        let entries = self.entries.read();
        let current = entries
            .get(name)
            .ok_or_else(|| Self::prompt_not_found(name))?;
        if current.version == version {
            return Ok(current.clone());
        }
        if let Some(history) = self.version_log.read().get(name) {
            for item in history {
                if item.version == version
                    && let Some(content) = &item.content_snapshot
                {
                    let mut entry = current.clone();
                    entry.content = content.clone();
                    entry.version = item.version.clone();
                    entry.modified_at = item.timestamp;
                    return Ok(entry);
                }
            }
        }
        Err(AppError::NotFound(format!(
            "版本 {version} 不存在于提示词 {name}"
        )))
    }

    pub(super) fn prompt_diff(
        &self,
        name: &str,
        version_a: &str,
        version_b: &str,
    ) -> Result<String, AppError> {
        let entries = self.entries.read();
        let current = entries
            .get(name)
            .ok_or_else(|| Self::prompt_not_found(name))?;
        let content_for = |version: &str| {
            if current.version == version {
                Ok(current.content.clone())
            } else {
                self.version_log
                    .read()
                    .get(name)
                    .and_then(|history| history.iter().find(|item| item.version == version))
                    .and_then(|item| item.content_snapshot.clone())
                    .ok_or_else(|| AppError::NotFound(format!("版本 {version} 不存在")))
            }
        };
        let content_a = content_for(version_a)?;
        let content_b = content_for(version_b)?;
        Ok(similar::TextDiff::from_lines(&content_a, &content_b)
            .unified_diff()
            .header(
                &format!("{}[{}]", name, version_a),
                &format!("{}[{}]", name, version_b),
            )
            .to_string())
    }

    pub(super) fn validate_prompt_variables(
        &self,
        template_name: &str,
        variables: &HashSet<String>,
    ) -> Result<Vec<String>, AppError> {
        let entries = self.entries.read();
        let entry = entries
            .get(template_name)
            .ok_or_else(|| Self::prompt_not_found(template_name))?;
        missing_template_variables(&entry.content, variables)
            .map_err(|error| AppError::Internal(anyhow::anyhow!("模板变量解析失败: {error}")))
    }

    pub(super) fn prompt_dependency_graph(&self) -> DependencyGraph {
        let entries = self.entries.read();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for entry in entries.values() {
            nodes.push(GraphNode {
                name: entry.name.clone(),
                category: entry.category.clone(),
            });
            for dependency in &entry.dependencies {
                edges.push(GraphEdge {
                    from: entry.name.clone(),
                    to: dependency.clone(),
                });
            }
        }
        DependencyGraph { nodes, edges }
    }

    pub(super) fn render_prompt_ab(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<(String, String), AppError> {
        let current = self.render_prompt(template_name, variables, project_id)?;
        let original_content = self
            .original_contents
            .read()
            .get(template_name)
            .cloned()
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("提示词缺少原始内容：{template_name}"))
            })?;
        let mut context = Context::new();
        for (key, value) in variables {
            context.insert(key, value);
        }
        let mut tera = Tera::default();
        tera.add_raw_template(template_name, &original_content)
            .map_err(|error| AppError::Internal(anyhow::anyhow!("模板编译失败: {error}")))?;
        let original = tera
            .render(template_name, &context)
            .map_err(|error| AppError::Internal(anyhow::anyhow!("渲染失败: {error}")))?;
        Ok((current, original))
    }

    pub(super) fn prompt_registry(&self) -> Arc<RwLock<crate::registry::PromptRegistry>> {
        self.registry.clone()
    }
}
