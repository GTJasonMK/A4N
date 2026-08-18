use a4n_core::error::AppError;
use serde_json::Value;

use crate::structured_llm::parse_strict_json_response;

use super::{AgentLoopAction, AgentLoopDecision};

pub(in crate::optimize) fn parse_agent_loop_decision_response(
    raw: &str,
) -> Result<AgentLoopDecision, AppError> {
    let value = parse_strict_json_response(raw, "content_optimization_agent_loop")?;
    parse_agent_loop_decision_value(&value)
}

fn parse_agent_loop_decision_value(value: &Value) -> Result<AgentLoopDecision, AppError> {
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
