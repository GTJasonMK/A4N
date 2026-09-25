use super::MAX_OBSERVATION_BYTES;
use a4n_agent_runtime_contract::AgentExecutionSnapshot;
use a4n_api_contract::agent::AgentResourceRef;
use a4n_domain::{generation::canonical_json_text, source_text_hash};
use a4n_error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(super) const OBSERVATION_VERSION: u32 = 1;
const RETRY_LIMIT: u8 = 3;

pub(super) fn error_value(value: &Value) -> Option<&Value> {
    value.get("error").filter(|error| !error.is_null())
}

pub(super) fn execution_result(execution: &AgentExecutionSnapshot) -> Result<Value, AppError> {
    let invalid = || AppError::InvalidState {
        current: format!(
            "Agent execution {} ({}) 缺少有效结果或错误回执",
            execution.id, execution.status
        ),
        operation: "恢复工具结果，禁止猜测或重复执行".into(),
    };
    if execution.status == "succeeded" {
        return execution.result_json.clone().ok_or_else(invalid);
    }
    if !matches!(
        execution.status.as_str(),
        "failed" | "rejected" | "cancelled"
    ) {
        return Err(invalid());
    }
    let mut value = execution.result_json.clone().unwrap_or_else(|| json!({}));
    if error_value(&value).is_none() {
        let message = execution.error_message.as_deref().ok_or_else(invalid)?;
        value
            .as_object_mut()
            .ok_or_else(invalid)?
            .insert("error".into(), Value::String(message.into()));
    }
    Ok(value)
}

pub fn bounded_observation(execution_id: i64, value: Value) -> Result<Value, AppError> {
    let size = serde_json::to_vec(&value)?.len();
    if size <= MAX_OBSERVATION_BYTES {
        return Ok(value);
    }
    Ok(json!({
        "execution_id": execution_id.to_string(),
        "result_bytes": size,
        "complete": false,
        "name": value.get("name").and_then(Value::as_str).filter(|name| name.len() < 200),
        "read_tool": "read_agent_execution_result",
        "message": "结果较大，已保存完整结果。按需使用 execution_id、JSON pointer、offset 和 limit 继续读取。"
    }))
}

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
) -> Result<Value, AppError> {
    let result_bytes = serde_json::to_vec(&value)?.len();
    let result_refs = super::tool_result_refs(tool_name, &value)?;
    let failure = error_value(&value).map(|error| {
        let message = value_text(error);
        let code = value
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("TOOL_ERROR")
            .to_string();
        (message, code)
    });
    let value = bounded_observation(execution_id, value)?;
    let partial = value.get("complete").and_then(Value::as_bool) == Some(false)
        && value.get("read_tool").is_some();
    let (status, text, error, data) = if let Some((message, code)) = failure {
        let (retryable, next_action) = retry_policy(&code);
        let (text, details) = if partial {
            (
                format!(
                    "工具 {tool_name} 失败，完整错误详情可通过 read_agent_execution_result 读取"
                ),
                value,
            )
        } else {
            (
                format!("工具 {tool_name} 失败：{message}"),
                json!({"message": message}),
            )
        };
        (
            "error".to_string(),
            text,
            Some(AgentToolObservationError {
                code,
                retryable,
                next_action,
                details,
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

pub fn tool_error_code(error: &AppError) -> &'static str {
    match error {
        AppError::NotFound(_) => "RESOURCE_NOT_FOUND",
        AppError::Forbidden(_) | AppError::Unauthorized(_) => "FORBIDDEN",
        AppError::InvalidArgument(_) => "INVALID_ARGUMENT",
        AppError::InvalidState { .. } => "CONFLICT",
        AppError::Timeout(_) => "TIMEOUT",
        AppError::Serialization(_) => "PROTOCOL_ERROR",
        _ => "TOOL_ERROR",
    }
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

pub(super) fn retry_prefix(tool_name: &str, arguments: &Value) -> Result<String, AppError> {
    let identity = json!({
        "tool": tool_name,
        "arguments": arguments,
    });
    Ok(source_text_hash(&canonical_json_text(&identity)?))
}

pub(super) fn restored_retry_counts(
    executions: &[AgentExecutionSnapshot],
) -> Result<HashMap<String, u8>, AppError> {
    let mut counts: HashMap<String, u8> = HashMap::new();
    for execution in executions {
        let (Some(tool_name), Some(arguments)) = (
            execution.tool_name.as_deref(),
            execution.arguments_json.as_ref(),
        ) else {
            continue;
        };
        let fingerprint = retry_prefix(tool_name, arguments)?;
        match execution.status.as_str() {
            "succeeded" => {
                counts.remove(&fingerprint);
            }
            "failed" => {
                let entry = counts.entry(fingerprint).or_insert(0);
                *entry = (*entry).saturating_add(1);
            }
            _ => {}
        }
    }
    Ok(counts)
}

pub(super) async fn retry_limit_reached(
    counts: &Arc<Mutex<HashMap<String, u8>>>,
    tool_name: &str,
    arguments: &Value,
) -> Result<bool, AppError> {
    let prefix = retry_prefix(tool_name, arguments)?;
    Ok(counts
        .lock()
        .await
        .get(&prefix)
        .is_some_and(|count| *count >= RETRY_LIMIT))
}

pub(super) async fn record_success(
    counts: &Arc<Mutex<HashMap<String, u8>>>,
    tool_name: &str,
    arguments: &Value,
) -> Result<(), AppError> {
    counts
        .lock()
        .await
        .remove(&retry_prefix(tool_name, arguments)?);
    Ok(())
}

pub(super) async fn record_failure(
    counts: &Arc<Mutex<HashMap<String, u8>>>,
    tool_name: &str,
    arguments: &Value,
) -> Result<u8, AppError> {
    let fingerprint = retry_prefix(tool_name, arguments)?;
    let mut counts = counts.lock().await;
    let entry = counts.entry(fingerprint).or_insert(0);
    *entry = (*entry).saturating_add(1);
    Ok(*entry)
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
        AgentToolObservation, from_result, parse, record_failure, retry_limit_reached,
        retry_policy, retry_prefix, tool_error_code,
    };
    use serde_json::json;
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::Mutex;

    #[test]
    fn observation_distinguishes_success_partial_and_error() {
        let success: AgentToolObservation = serde_json::from_value(
            from_result(
                1,
                "read",
                json!({"items": [], "error": null, "resource": {"kind": "chapter", "id": "7"}}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(success.status, "success");
        assert!(success.error.is_none());
        assert_eq!(success.result_refs[0].id, "7");

        let partial: AgentToolObservation = serde_json::from_value(
            from_result(
                2,
                "read",
                json!({"complete": false, "read_tool": "read_agent_execution_result"}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(partial.status, "partial");
        assert!(!partial.execution.complete);

        let error: AgentToolObservation = serde_json::from_value(
            from_result(3, "edit", json!({"error": "版本冲突", "code": "CONFLICT"})).unwrap(),
        )
        .unwrap();
        assert_eq!(error.status, "error");
        assert_eq!(error.error.unwrap().next_action, "read_before_edit");
    }

    #[test]
    fn replay_uses_the_failure_receipt_and_rejects_missing_results() {
        let mut execution = a4n_agent_runtime_contract::AgentExecutionSnapshot {
            id: 7,
            run_id: 1,
            execution_key: "tool-call:7".into(),
            execution_kind: "tool_call".into(),
            status: "failed".into(),
            tool_name: Some("write".into()),
            effect: Some("write".into()),
            replay_policy: None,
            arguments_json: None,
            resource_preconditions_json: None,
            result_json: Some(json!({"error": null, "code": "FORBIDDEN"})),
            result_summary: None,
            result_refs_json: None,
            error_message: Some("用户拒绝".into()),
        };
        assert_eq!(
            super::execution_result(&execution).unwrap(),
            json!({"error": "用户拒绝", "code": "FORBIDDEN"})
        );
        execution.result_json = None;
        execution.error_message = None;
        assert!(super::execution_result(&execution).is_err());
        execution.status = "succeeded".into();
        assert!(super::execution_result(&execution).is_err());
        execution.result_json = Some(json!({"ok": true}));
        assert_eq!(
            super::execution_result(&execution).unwrap(),
            json!({"ok": true})
        );
    }

    #[test]
    fn large_failures_keep_the_error_code_and_a_bounded_read_handle() {
        let value = from_result(
            9,
            "write",
            json!({"error": "拒绝".repeat(super::MAX_OBSERVATION_BYTES), "code": "FORBIDDEN"}),
        )
        .unwrap();
        let observation = parse(&value).unwrap();
        assert_eq!(observation.status, "error");
        assert!(!observation.execution.complete);
        let error = observation.error.unwrap();
        assert_eq!(error.code, "FORBIDDEN");
        assert_eq!(error.details["execution_id"], "9");
        assert_eq!(error.details["read_tool"], "read_agent_execution_result");
        assert!(serde_json::to_vec(&value).unwrap().len() < super::MAX_OBSERVATION_BYTES);
    }

    #[test]
    fn errors_have_small_stable_codes_and_policies() {
        assert_eq!(
            tool_error_code(&a4n_error::AppError::NotFound(
                "revision_tool 不存在".into()
            )),
            "RESOURCE_NOT_FOUND"
        );
        assert_eq!(
            tool_error_code(&a4n_error::AppError::Forbidden(
                "不能读取不存在的资源".into()
            )),
            "FORBIDDEN"
        );
        assert_eq!(
            retry_policy("FORBIDDEN"),
            (false, "fix_arguments_or_report".to_string())
        );
    }

    #[test]
    fn unknown_observation_versions_are_rejected() {
        let value = from_result(1, "read", json!({"ok": true})).unwrap();
        let mut value = value;
        value["observation_version"] = json!(99);
        assert!(parse(&value).is_err());
    }

    #[tokio::test]
    async fn repeated_same_tool_arguments_hit_retry_limit() {
        let counts = Arc::new(Mutex::new(HashMap::new()));
        let arguments = json!({"project_id": 7});
        for expected in 1..=3 {
            let count = record_failure(&counts, "read_project", &arguments)
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
        super::record_success(&counts, "read_project", &arguments)
            .await
            .unwrap();
        assert!(
            !retry_limit_reached(&counts, "read_project", &arguments)
                .await
                .unwrap()
        );
        assert_eq!(
            record_failure(&counts, "read_project", &arguments)
                .await
                .unwrap(),
            1
        );
    }

    #[test]
    fn retry_fingerprint_uses_canonical_argument_order() {
        let first = retry_prefix("read_project", &json!({"a": 1, "b": 2})).unwrap();
        let second = retry_prefix("read_project", &json!({"b": 2, "a": 1})).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn outcome_unknown_is_an_error_observation_not_success() {
        let observation: AgentToolObservation = serde_json::from_value(
            from_result(
                9,
                "save_editor_draft",
                json!({"error": "等待对账", "code": "OUTCOME_UNKNOWN"}),
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
