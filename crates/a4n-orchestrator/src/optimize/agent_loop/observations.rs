use a4n_error::AppError;
use serde_json::{Value, json};

use super::decision::reject_unknown_action_argument_fields;
use super::{AgentLoopAction, ThinkingObservation};

pub(in crate::optimize) fn thinking_observation(
    step: usize,
    thought: &str,
    arguments: &Value,
) -> Result<ThinkingObservation, AppError> {
    reject_unknown_action_argument_fields(
        arguments,
        &["focus", "known_facts", "information_gaps", "next_step"],
        "think",
    )?;
    let focus = required_string(arguments, "focus")?;
    let known_facts = required_string_array(arguments, "known_facts")?;
    let information_gaps = required_string_array(arguments, "information_gaps")?;
    let next_step = required_string(arguments, "next_step")?;
    let gap_summary = if information_gaps.is_empty() {
        "无".to_string()
    } else {
        information_gaps.join("；")
    };
    let event_summary = format!("聚焦：{focus}；信息缺口：{gap_summary}；下一步：{next_step}");
    let value = json!({
        "step": step,
        "action": AgentLoopAction::Think.as_str(),
        "thought": thought,
        "focus": focus,
        "known_facts": known_facts,
        "information_gaps": information_gaps,
        "next_step": next_step,
        "success": true,
    });
    Ok(ThinkingObservation {
        value,
        event_summary,
    })
}

fn required_string(arguments: &Value, key: &str) -> Result<String, AppError> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("think 缺少非空 {key}")))
}

fn required_string_array(arguments: &Value, key: &str) -> Result<Vec<String>, AppError> {
    let values = arguments
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("think.{key} 必须是字符串数组")))?;
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("think.{key}[{index}] 必须是非空字符串"))
                })
        })
        .collect()
}

pub(in crate::optimize) fn insert_current_chapter_content(context: &mut Value, content: &str) {
    if let Some(current_chapter) = context
        .get_mut("current_chapter")
        .and_then(Value::as_object_mut)
    {
        current_chapter.insert("content".to_string(), json!(content));
    }
}

pub(in crate::optimize) fn run_tool_arguments(
    arguments: &Value,
) -> Result<(&str, Value), AppError> {
    reject_unknown_action_argument_fields(arguments, &["tool", "arguments"], "run_tool")?;
    let tool = arguments
        .get("tool")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("run_tool 缺少 tool".to_string()))?;
    let tool_arguments = arguments
        .get("arguments")
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("run_tool 缺少 arguments".to_string()))?;
    if !tool_arguments.is_object() {
        return Err(AppError::InvalidArgument(
            "run_tool.arguments 必须是对象".to_string(),
        ));
    }
    Ok((tool, tool_arguments))
}

pub(in crate::optimize) fn final_issues(arguments: &Value) -> Result<Value, AppError> {
    reject_unknown_action_argument_fields(arguments, &["issues"], "final")?;
    let issues = arguments
        .get("issues")
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument("final 缺少 issues".to_string()))?;
    if !issues.is_array() {
        return Err(AppError::InvalidArgument(
            "final.issues 必须是数组".to_string(),
        ));
    }
    Ok(issues)
}

pub(in crate::optimize) fn final_reply(arguments: &Value) -> Result<String, AppError> {
    reject_unknown_action_argument_fields(arguments, &["reply"], "final")?;
    arguments
        .get("reply")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument("final 缺少非空 reply".to_string()))
}

pub(in crate::optimize) fn tool_observation(
    step: usize,
    action: &AgentLoopAction,
    tool: &str,
    arguments: Value,
    result: Value,
) -> Value {
    json!({
        "step": step,
        "action": action.as_str(),
        "tool": tool,
        "arguments": arguments,
        "success": true,
        "result": result,
    })
}

pub(in crate::optimize) fn failed_tool_observation(
    step: usize,
    action: &AgentLoopAction,
    tool: &str,
    arguments: Value,
    err: &AppError,
) -> Value {
    json!({
        "step": step,
        "action": action.as_str(),
        "tool": tool,
        "arguments": arguments,
        "success": false,
        "error": err.to_string(),
    })
}

pub(in crate::optimize) fn final_response_from_issues(issues: Value) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(&json!({"issues": issues}))?)
}
