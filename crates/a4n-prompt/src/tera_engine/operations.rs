use std::collections::{HashMap, HashSet};

use tera::{Context, Tera};

use a4n_core::error::AppError;

use crate::engine::{PromptCategory, PromptEntry, PromptVersion};

use super::state::TeraPromptEngine;
use super::variables::missing_template_variables;

impl TeraPromptEngine {
    pub fn render(
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

    pub fn get(&self, name: &str) -> Result<PromptEntry, AppError> {
        self.entries
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| Self::prompt_not_found(name))
    }

    pub fn list_all(&self) -> Vec<PromptEntry> {
        self.entries.read().values().cloned().collect()
    }

    pub fn version_history(&self, name: &str) -> Result<Vec<PromptVersion>, AppError> {
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

    pub fn diff(&self, name: &str, version_a: &str, version_b: &str) -> Result<String, AppError> {
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

    pub fn validate_variables(
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

    pub fn categories(&self) -> Result<Vec<PromptCategory>, AppError> {
        let registry = self.registry.read();
        let mut categories = Vec::with_capacity(registry.categories().len());
        for (key, value) in registry.categories() {
            categories.push(PromptCategory {
                key: key.clone(),
                label: value.label.clone(),
                order: value
                    .order
                    .ok_or_else(|| AppError::ConfigError(format!("提示词分类 {key} 缺少 order")))?,
                description: value.description.clone(),
            });
        }
        categories.sort_by_key(|category| (category.order, category.key.clone()));
        Ok(categories)
    }

    pub fn render_ab(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<(String, String), AppError> {
        let current = self.render(template_name, variables, project_id)?;
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
}
