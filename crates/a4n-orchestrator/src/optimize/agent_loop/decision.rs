use a4n_core::error::AppError;
use serde_json::Value;

use super::{AgentLoopAction, AgentLoopDecision};

#[cfg(test)]
pub(in crate::optimize) fn parse_agent_loop_decision(
    raw: &str,
) -> Result<AgentLoopDecision, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization_agent_loop 模型返回空内容，无法解析为 JSON 决策".to_string(),
        ));
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|err| {
        AppError::InvalidArgument(format!(
            "content_optimization_agent_loop 必须返回严格 JSON 决策对象: {err}"
        ))
    })?;
    parse_agent_loop_decision_value(&value)
}

pub(in crate::optimize) fn agent_loop_decision_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["thought", "public_update", "action", "arguments"],
        "properties": {
            "thought": {"type": "string"},
            "public_update": {"type": ["string", "null"]},
            "action": {"enum": ["think", "run_tool", "final"]},
            "arguments": {"type": "object"}
        }
    })
}

pub(in crate::optimize) fn parse_agent_loop_decision_value(
    value: &Value,
) -> Result<AgentLoopDecision, AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument(
            "content_optimization_agent_loop 输出必须是 JSON 对象".to_string(),
        )
    })?;
    reject_unknown_decision_fields(object)?;
    let thought = object
        .get("thought")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("Agent Loop 决策缺少非空 thought".to_string()))?
        .to_string();
    let action = object
        .get("action")
        .and_then(Value::as_str)
        .map(str::trim)
        .ok_or_else(|| AppError::InvalidArgument("Agent Loop 决策缺少 action".to_string()))?;
    let action = match action {
        "think" => AgentLoopAction::Think,
        "run_tool" => AgentLoopAction::RunTool,
        "final" => AgentLoopAction::Final,
        other => {
            return Err(AppError::InvalidArgument(format!(
                "Agent Loop 决策 action 非法: {other}"
            )));
        }
    };
    let public_update = object
        .get("public_update")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if action != AgentLoopAction::Final && public_update.is_none() {
        return Err(AppError::InvalidArgument(
            "Agent Loop 非 final 决策缺少非空 public_update".to_string(),
        ));
    }
    let arguments = object
        .get("arguments")
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("Agent Loop 决策缺少 arguments".to_string()))?;
    if !arguments.is_object() {
        return Err(AppError::InvalidArgument(
            "Agent Loop 决策 arguments 必须是对象".to_string(),
        ));
    }

    Ok(AgentLoopDecision {
        thought,
        public_update,
        action,
        arguments,
    })
}

fn reject_unknown_decision_fields(object: &serde_json::Map<String, Value>) -> Result<(), AppError> {
    let allowed = ["thought", "public_update", "action", "arguments"];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "Agent Loop 决策包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

pub(super) fn reject_unknown_action_argument_fields(
    arguments: &Value,
    allowed: &[&str],
    context: &str,
) -> Result<(), AppError> {
    let object = arguments.as_object().ok_or_else(|| {
        AppError::InvalidArgument(format!("Agent Loop {context}.arguments 必须是对象"))
    })?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "Agent Loop {context}.arguments 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}
