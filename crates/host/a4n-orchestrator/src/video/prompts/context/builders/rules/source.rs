use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Map, Value, json};

use a4n_prompt::rendering::PromptRenderer;

const VIDEO_BUILDER_RULES_TEMPLATE: &str = "chapter_video_builder_rules";

pub(in crate::video::prompts::context::builders) async fn video_builder_rules(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
) -> Result<Map<String, Value>, AppError> {
    let rendered = prompt_renderer
        .render_prompt(
            VIDEO_BUILDER_RULES_TEMPLATE,
            &HashMap::new(),
            Some(project_id),
        )
        .await?;
    serde_json::from_str::<Value>(&rendered)
        .map_err(|err| {
            AppError::ConfigError(format!(
                "视频 builder 规则模板必须渲染为合法 JSON 对象：{err}"
            ))
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| {
            AppError::ConfigError("视频 builder 规则模板必须渲染为 JSON 对象".to_string())
        })
}

pub(in crate::video::prompts::context::builders) fn rule_value(
    rules: &Map<String, Value>,
    key: &str,
) -> Result<Value, AppError> {
    rules
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::ConfigError(format!("视频 builder 规则模板缺少必需字段：{key}")))
}

pub(in crate::video::prompts::context::builders) fn rule_text(
    rules: &Map<String, Value>,
    key: &str,
) -> Result<String, AppError> {
    rule_value(rules, key)?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| {
            AppError::ConfigError(format!("视频 builder 规则模板字段必须是字符串：{key}"))
        })
}

pub(in crate::video::prompts::context::builders) fn insert_rule_value(
    constraints: &mut Map<String, Value>,
    constraints_key: &str,
    rules: &Map<String, Value>,
    rule_key: &str,
) -> Result<(), AppError> {
    constraints.insert(constraints_key.to_string(), rule_value(rules, rule_key)?);
    Ok(())
}

pub(in crate::video::prompts::context::builders) fn video_prompt_task(
    rules: &Map<String, Value>,
    name: &str,
    instruction_key: &str,
) -> Result<Value, AppError> {
    Ok(json!({
        "name": name,
        "instruction": rule_text(rules, instruction_key)?
    }))
}
