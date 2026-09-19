use a4n_agent_runtime_contract::AgentExecutionSnapshot;
use a4n_api_contract::agent::AgentResourceRef;
use a4n_domain::source_text_hash;
use a4n_error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(super) const OBSERVATION_VERSION: u32 = 1;
const RETRY_LIMIT: u8 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentToolObservation {
    pub observation_version: u32,
    pub status: String,
    pub data: Value,
    pub text: String,
    pub error: Option<AgentToolObservationError>,
    pub result_refs: Vec<AgentResourceRef>,
    pub execution: AgentToolObservationExecution,
    pub stats: AgentToolObservationStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentToolObservationError {
    pub code: String,
    pub retryable: bool,
    pub next_action: String,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentToolObservationExecution {
    pub id: String,
    pub complete: bool,
    pub read_tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentToolObservationStats {
    pub duration_ms: u64,
    pub result_bytes: usize,
}

pub(super) fn parse(value: &Value) -> Result<AgentToolObservation, AppError> {
    let observation: AgentToolObservation = serde_json::from_value(value.clone())?;
    if observation.observation_version != OBSERVATION_VERSION {
        return Err(AppError::InvalidArgument(format!(
            "不支持的 Agent tool observation 版本: {}",
            observation.observation_version
        )));
    }
    Ok(observation)
}

pub(super) fn from_result(
    execution_id: i64,
    tool_name: &str,
    value: Value,
    result_refs: Vec<AgentResourceRef>,
    result_bytes: usize,
) -> Result<Value, AppError> {
    let partial = value.get("complete").and_then(Value::as_bool) == Some(false)
        && value.get("read_tool").is_some();
    let error_text = value.get("error").map(value_text);
    let (status, text, error, data) = if let Some(message) = error_text {
        let code = value
            .get("code")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| error_code(&message));
        let (retryable, next_action) = retry_policy(&code);
        (
            "error".to_string(),
            format!("工具 {tool_name} 失败：{message}"),
            Some(AgentToolObservationError {
                code,
                retryable,
                next_action,
                details: json!({"message": message}),
            }),
            Value::Null,
        )
    } else if partial {
        (
            "partial".to_string(),
            format!(
                "工具 {tool_name} 返回了截断结果，可通过 read_agent_execution_result 读取完整结果"
            ),
            None,
            value,
        )
    } else {
        let text = if value
            .get("items")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            format!("工具 {tool_name} 查询成功，但没有匹配项")
        } else {
            format!("工具 {tool_name} 执行成功")
        };
        ("success".to_string(), text, None, value)
    };
    serde_json::to_value(AgentToolObservation {
        observation_version: OBSERVATION_VERSION,
        status,
        data,
        text,
        error,
        result_refs,
        execution: AgentToolObservationExecution {
            id: execution_id.to_string(),
            complete: !partial,
            read_tool: partial.then(|| "read_agent_execution_result".into()),
        },
        stats: AgentToolObservationStats {
            duration_ms: 0,
            result_bytes,
        },
    })
    .map_err(AppError::from)
}

pub(super) fn error_code(message: &str) -> String {
    let value = message.to_ascii_lowercase();
    if value.contains("outcome_unknown") || value.contains("对账") {
        "OUTCOME_UNKNOWN"
    } else if value.contains("conflict") || value.contains("版本") || value.contains("revision") {
        "CONFLICT"
    } else if value.contains("不存在") || value.contains("not found") {
        "RESOURCE_NOT_FOUND"
    } else if value.contains("权限") || value.contains("禁止") || value.contains("forbidden") {
        "FORBIDDEN"
    } else if value.contains("审批") || value.contains("approval") {
        "APPROVAL_REQUIRED"
    } else if value.contains("超时") || value.contains("timeout") {
        "TIMEOUT"
    } else if value.contains("不可用") || value.contains("unavailable") || value.contains("连接")
    {
        "EXTERNAL_UNAVAILABLE"
    } else if value.contains("协议") || value.contains("schema") || value.contains("解码") {
        "PROTOCOL_ERROR"
    } else {
        "INVALID_ARGUMENT"
    }
    .into()
}

pub(super) fn retry_policy(code: &str) -> (bool, String) {
    let (retryable, next_action) = match code {
        "CONFLICT" => (true, "read_before_edit"),
        "RESOURCE_NOT_FOUND" => (true, "check_scope_or_identifier"),
        "EXTERNAL_UNAVAILABLE" | "TIMEOUT" => (true, "wait_or_report"),
        "PROTOCOL_ERROR" => (true, "retry_once_with_correct_shape"),
        _ => (false, "fix_arguments_or_report"),
    };
    (retryable, next_action.into())
}

pub(super) fn retry_fingerprint(
    tool_name: &str,
    arguments: &Value,
    code: &str,
) -> Result<String, AppError> {
    Ok(format!("{}:{code}", retry_prefix(tool_name, arguments)?))
}

pub(super) fn retry_prefix(tool_name: &str, arguments: &Value) -> Result<String, AppError> {
    Ok(source_text_hash(&serde_json::to_string(&json!({
        "tool": tool_name,
        "arguments": arguments,
    }))?))
}

pub(super) fn restored_retry_counts(
    executions: &[AgentExecutionSnapshot],
) -> Result<HashMap<String, u8>, AppError> {
    let mut counts: HashMap<String, u8> = HashMap::new();
    for execution in executions
        .iter()
        .filter(|execution| execution.status == "failed")
    {
        let (Some(tool_name), Some(arguments), Some(message)) = (
            execution.tool_name.as_deref(),
            execution.arguments_json.as_ref(),
            execution.error_message.as_deref(),
        ) else {
            continue;
        };
        let code = error_code(message);
        let fingerprint = retry_fingerprint(tool_name, arguments, &code)?;
        let entry = counts.entry(fingerprint).or_insert(0);
        *entry = (*entry).saturating_add(1);
    }
    Ok(counts)
}

pub(super) async fn retry_limit_reached(
    counts: &Arc<Mutex<HashMap<String, u8>>>,
    tool_name: &str,
    arguments: &Value,
) -> Result<bool, AppError> {
    let prefix = retry_prefix(tool_name, arguments)?;
    Ok(counts.lock().await.iter().any(|(fingerprint, count)| {
        *count >= RETRY_LIMIT && fingerprint.starts_with(&format!("{prefix}:"))
    }))
}

pub(super) async fn record_failure(
    counts: &Arc<Mutex<HashMap<String, u8>>>,
    tool_name: &str,
    arguments: &Value,
    message: &str,
) -> Result<(String, u8), AppError> {
    let code = error_code(message);
    let fingerprint = retry_fingerprint(tool_name, arguments, &code)?;
    let mut counts = counts.lock().await;
    let entry = counts.entry(fingerprint).or_insert(0);
    *entry = (*entry).saturating_add(1);
    Ok((code, *entry))
}

fn value_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        AgentToolObservation, error_code, from_result, parse, record_failure, retry_limit_reached,
        retry_policy,
    };
    use serde_json::json;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;

    #[test]
    fn observation_distinguishes_success_partial_and_error() {
        let success: AgentToolObservation = serde_json::from_value(
            from_result(1, "read", json!({"items": []}), Vec::new(), 12).unwrap(),
        )
        .unwrap();
        assert_eq!(success.status, "success");
        assert!(success.error.is_none());

        let partial: AgentToolObservation = serde_json::from_value(
            from_result(
                2,
                "read",
                json!({"complete": false, "read_tool": "read_agent_execution_result"}),
                Vec::new(),
                20_000,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(partial.status, "partial");
        assert!(!partial.execution.complete);

        let error: AgentToolObservation = serde_json::from_value(
            from_result(
                3,
                "edit",
                json!({"error": "版本冲突", "code": "CONFLICT"}),
                Vec::new(),
                20,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(error.status, "error");
        assert_eq!(error.error.unwrap().next_action, "read_before_edit");
    }

    #[test]
    fn errors_have_small_stable_codes_and_policies() {
        assert_eq!(error_code("资源不存在"), "RESOURCE_NOT_FOUND");
        assert_eq!(
            retry_policy("FORBIDDEN"),
            (false, "fix_arguments_or_report".to_string())
        );
    }

    #[test]
    fn unknown_observation_versions_are_rejected() {
        let value = from_result(1, "read", json!({"ok": true}), Vec::new(), 10).unwrap();
        let mut value = value;
        value["observation_version"] = json!(99);
        assert!(parse(&value).is_err());
    }

    #[tokio::test]
    async fn repeated_same_tool_arguments_hit_retry_limit() {
        let counts = Arc::new(Mutex::new(HashMap::new()));
        let arguments = json!({"project_id": 7});
        for expected in 1..=3 {
            let (_, count) = record_failure(&counts, "read_project", &arguments, "资源不存在")
                .await
                .expect("failure should be recorded");
            assert_eq!(count, expected);
        }
        assert!(
            retry_limit_reached(&counts, "read_project", &arguments)
                .await
                .expect("retry limit should be readable")
        );
        assert!(
            !retry_limit_reached(&counts, "read_project", &json!({"project_id": 8}))
                .await
                .expect("different arguments should remain retryable")
        );
    }

    #[test]
    fn outcome_unknown_is_an_error_observation_not_success() {
        let observation: AgentToolObservation = serde_json::from_value(
            from_result(
                9,
                "save_editor_draft",
                json!({"error": "等待对账", "code": "OUTCOME_UNKNOWN"}),
                Vec::new(),
                32,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(observation.status, "error");
        assert_eq!(
            observation.error.as_ref().map(|error| error.code.as_str()),
            Some("OUTCOME_UNKNOWN")
        );
        assert!(!observation.error.unwrap().retryable);
    }
}
