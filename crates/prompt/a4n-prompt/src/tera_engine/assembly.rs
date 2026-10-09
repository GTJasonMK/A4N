use std::collections::HashMap;

use a4n_error::AppError;
use a4n_prompt_contract::{PromptAssemblyBlockKind, PromptAssemblyConfig, PromptAssemblySnapshot};
use serde_json::Value;
use tera::{Context, Tera};

use super::TeraPromptEngine;
use super::variables::required_template_variables;

impl TeraPromptEngine {
    pub fn registry_assembly_call_keys(&self, prompt_name: &str) -> Vec<String> {
        self.registry
            .read()
            .get(prompt_name)
            .map(|entry| entry.assembly_call_keys.clone())
            .unwrap_or_default()
    }

    pub fn template_variables(
        &self,
        prompt_name: &str,
        project_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        let (content, _) = self.resolve_content(prompt_name, project_id)?;
        required_template_variables(&content)
            .map_err(|error| AppError::InvalidArgument(format!("提示词变量解析失败：{error}")))
    }

    pub fn render_assembled(
        &self,
        snapshot: &PromptAssemblySnapshot,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        snapshot
            .config
            .validate()
            .map_err(AppError::InvalidArgument)?;
        let mut context = Context::new();
        for (key, value) in variables {
            context.insert(key, value);
        }

        let mut blocks = Vec::new();
        for block in snapshot.config.blocks.iter().filter(|block| block.enabled) {
            let rendered = match block.kind {
                PromptAssemblyBlockKind::Template => {
                    let required = self.template_variables(&block.value, project_id)?;
                    require_variables(&required, variables, &block.id)?;
                    self.render_template(&block.value, variables, project_id)?
                }
                PromptAssemblyBlockKind::Variable => {
                    let value = variables.get(&block.value).ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "提示词装配变量 {} 不可用（模块 {}）",
                            block.value, block.id
                        ))
                    })?;
                    match value {
                        Value::String(value) => value.clone(),
                        value => {
                            serde_json::to_string_pretty(value).map_err(AppError::Serialization)?
                        }
                    }
                }
                PromptAssemblyBlockKind::Custom => {
                    let required = required_template_variables(&block.value).map_err(|error| {
                        AppError::InvalidArgument(format!(
                            "自定义装配模块 {} 的模板无效：{error}",
                            block.id
                        ))
                    })?;
                    require_variables(&required, variables, &block.id)?;
                    let mut tera = Tera::default();
                    tera.render_str(&block.value, &context).map_err(|error| {
                        AppError::InvalidArgument(format!(
                            "自定义装配模块 {} 渲染失败：{error}",
                            block.id
                        ))
                    })?
                }
            };
            if !rendered.trim().is_empty() {
                blocks.push(rendered.trim().to_string());
            }
        }
        if blocks.is_empty() {
            return Err(AppError::InvalidArgument(
                "提示词装配没有生成任何内容".into(),
            ));
        }
        Ok(blocks.join("\n\n"))
    }

    pub fn registry_assembly_definitions(
        &self,
        prompt_name: &str,
    ) -> Option<Vec<a4n_prompt_contract::PromptAssemblyBlock>> {
        self.registry.read().get(prompt_name).map(|entry| {
            entry
                .assembly_blocks
                .iter()
                .map(|block| a4n_prompt_contract::PromptAssemblyBlock {
                    id: block.id.clone(),
                    kind: block.kind,
                    value: block.value.clone(),
                    enabled: block.enabled,
                })
                .collect()
        })
    }

    pub fn registry_assembly_metadata(
        &self,
        prompt_name: &str,
    ) -> Option<Vec<(String, String, bool, Vec<String>)>> {
        self.registry.read().get(prompt_name).map(|entry| {
            entry
                .assembly_blocks
                .iter()
                .map(|block| {
                    (
                        block.id.clone(),
                        block.label.clone(),
                        block.protected,
                        block.required_variables.clone(),
                    )
                })
                .collect()
        })
    }

    pub fn default_assembly(&self, prompt_name: &str) -> PromptAssemblyConfig {
        PromptAssemblyConfig::system_default(prompt_name)
    }
}

fn require_variables(
    required: &[String],
    variables: &HashMap<String, Value>,
    block_id: &str,
) -> Result<(), AppError> {
    let missing = required
        .iter()
        .filter(|name| !variables.contains_key(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "提示词装配模块 {block_id} 缺少变量：{}",
            missing.join("、")
        )))
    }
}
