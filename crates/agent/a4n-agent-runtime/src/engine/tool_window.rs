use super::*;
use std::sync::atomic::AtomicI64;

pub(crate) struct RuntimeToolWindow {
    pub(crate) schemas: Vec<Value>,
    pub(crate) dropped_optional: Vec<String>,
}

pub(crate) fn available_runtime_tools(
    capability: &str,
    scope: &AgentScope,
    discovered_tools: &[String],
) -> Result<RuntimeToolWindow, AppError> {
    let initial_tools = a4n_api_contract::agent::initial_agent_tools(capability);
    let foundation = [
        "request_agent_input",
        "search_agent_tools",
        "describe_agent_tool",
        "search_agent_skills",
        "load_agent_skill",
        "read_agent_execution_result",
    ];
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    let mut dropped_optional = Vec::new();
    // Pending calls are resumed from their executions before another model
    // request. Their schemas need no second, potentially stale, mandatory list.
    for name in foundation.into_iter().chain(initial_tools.iter().copied()) {
        if !seen.insert(name)
            || ensure_agent_scope_allows_tool(scope, name).is_err()
            || !a4n_api_contract::agent::capability_allows_tool(capability, name)
        {
            continue;
        }
        names.push(name);
    }
    for name in discovered_tools.iter().rev().map(String::as_str) {
        if !seen.insert(name)
            || ensure_agent_scope_allows_tool(scope, name).is_err()
            || !a4n_api_contract::agent::capability_allows_tool(capability, name)
        {
            continue;
        }
        if names.len() >= MAX_AGENT_TOOL_SCHEMAS {
            dropped_optional.push(name.to_string());
        } else {
            names.push(name);
        }
    }
    names
        .iter()
        .map(|name| {
            let descriptor = tool_descriptor(name)?;
            Ok(json!({
                "type": "function",
                "function": {
                    "name": descriptor.name,
                    "description": descriptor.summary,
                    "parameters": descriptor.args_schema,
                }
            }))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|schemas| RuntimeToolWindow {
            schemas,
            dropped_optional,
        })
}

pub(crate) async fn record_tool_failure(
    execution_application: &dyn a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort,
    message_application: &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort,
    claimed: &AgentWorkerRun,
    execution: &AgentExecutionSnapshot,
    call: &ToolCall,
    error: AppError,
    messages: &mut Vec<ChatMessage>,
    latest_message_id: &AtomicI64,
) -> Result<(RunProgress, Vec<String>), AppError> {
    let message = error.to_string();
    let value = json!({"error": message, "code": observation::tool_error_code(&error)});
    if execution.status == "awaiting_user" {
        execution_application
            .update_worker_execution(WorkerExecutionUpdate {
                run_id: claimed.run.id,
                lease_token: claimed.lease_token.clone(),
                execution_id: execution.id,
                status: "dispatching".into(),
                result_json: None,
                result_summary: None,
                result_refs_json: None,
                error_message: None,
            })
            .await?;
    }
    execution_application
        .update_worker_execution(WorkerExecutionUpdate {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_id: execution.id,
            status: "failed".into(),
            result_json: Some(value.clone()),
            result_summary: Some(message.clone()),
            result_refs_json: None,
            error_message: Some(message.clone()),
        })
        .await?;
    append_tool_observation(
        message_application,
        claimed,
        execution,
        call,
        value,
        messages,
        latest_message_id,
    )
    .await?;
    Ok((RunProgress::Continue, Vec::new()))
}

pub(crate) fn discovered_tool_names(
    tool_name: &str,
    result: &Option<Value>,
) -> Result<Vec<String>, AppError> {
    let Some(result) = result else {
        return Ok(Vec::new());
    };
    let observation = result
        .get("observation_version")
        .map(|_| observation::parse(result))
        .transpose()?;
    if observation
        .as_ref()
        .is_some_and(|observation| observation.error.is_some())
    {
        return Ok(Vec::new());
    }
    let result = observation
        .as_ref()
        .map_or(result, |observation| &observation.data);
    if observation::error_value(result).is_some() {
        return Ok(Vec::new());
    }
    match tool_name {
        "describe_agent_tool"
            if result.get("read_tool").and_then(Value::as_str)
                == Some("read_agent_execution_result") =>
        {
            result
                .get("name")
                .and_then(Value::as_str)
                .map(|name| vec![name.to_string()])
                .ok_or_else(|| AppError::InvalidState {
                    current: "已保存的工具描述缺少名称".into(),
                    operation: "恢复工具目录".into(),
                })
        }
        "describe_agent_tool" => serde_json::from_value::<AgentToolDescriptor>(result.clone())
            .map(|tool| vec![tool.name])
            .map_err(|error| AppError::InvalidState {
                current: format!("Agent 工具 {tool_name} 返回的工具描述无效: {error}"),
                operation: "恢复 Agent 工具发现结果".into(),
            }),
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn is_discovery_tool(name: &str) -> bool {
    matches!(
        name,
        "search_agent_tools" | "describe_agent_tool" | "search_agent_skills" | "load_agent_skill"
    )
}

pub(crate) fn discovery_key(name: &str, arguments: &Value) -> Option<String> {
    is_discovery_tool(name).then(|| format!("{name}:{arguments}"))
}

pub(crate) fn successful_discovery_cache(
    executions: &[AgentExecutionSnapshot],
) -> HashMap<String, Value> {
    let mut cache = HashMap::new();
    for execution in executions {
        if execution.status != "succeeded" {
            continue;
        }
        let Some(tool_name) = execution.tool_name.as_deref() else {
            continue;
        };
        let Some(arguments) = execution.arguments_json.as_ref() else {
            continue;
        };
        let Some(result) = execution.result_json.clone() else {
            continue;
        };
        if let Some(key) = discovery_key(tool_name, arguments) {
            cache.insert(key, result);
        }
    }
    cache
}

pub(crate) fn extend_discovered_tools(
    discovered_tools: &mut Vec<String>,
    names: Vec<String>,
    scope: &AgentScope,
) -> Result<(), AppError> {
    for name in names {
        let name = name.trim();
        if name.is_empty() || ensure_agent_scope_allows_tool(scope, name).is_err() {
            continue;
        }
        // Re-resolve every discovered name through the canonical descriptor.
        // The model-facing result is not trusted as an executable schema.
        tool_descriptor(name)?;
        if !discovered_tools.iter().any(|known| known == name) {
            discovered_tools.push(name.to_string());
            if discovered_tools.len() > MAX_DISCOVERED_AGENT_TOOLS {
                discovered_tools.remove(0);
            }
        }
    }
    Ok(())
}

pub(crate) fn usage_object(value: &Value) -> Result<Value, AppError> {
    if value.is_object() {
        Ok(value.clone())
    } else {
        Err(AppError::InvalidState {
            current: "Agent usage_json 不是对象".into(),
            operation: "读取 Agent usage".into(),
        })
    }
}

pub fn json_u64(value: &Value, key: &str) -> Result<u64, AppError> {
    match value.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run".into(),
        }),
    }
}

pub fn enum_string<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .expect("Agent policy enum must serialize")
        .as_str()
        .expect("Agent policy enum must serialize to a string")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_restores_wrapped_observations_after_wait_or_restart() {
        let descriptor = tool_descriptor("get_project").unwrap();
        let mut raw = serde_json::to_value(&descriptor).unwrap();
        let policy = raw["agent_policy"].take();
        raw.as_object_mut()
            .unwrap()
            .extend(policy.as_object().unwrap().clone());
        let wrapped = observation::from_result(7, "describe_agent_tool", raw.clone()).unwrap();
        assert_eq!(
            discovered_tool_names("describe_agent_tool", &Some(raw)).unwrap(),
            ["get_project"]
        );
        assert_eq!(
            discovered_tool_names("describe_agent_tool", &Some(wrapped)).unwrap(),
            ["get_project"]
        );
        let failed =
            observation::from_result(8, "describe_agent_tool", json!({"error":"not found"}))
                .unwrap();
        assert!(
            discovered_tool_names("describe_agent_tool", &Some(failed))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn specialized_discovery_keeps_loaded_tools_and_resource_permissions() {
        for capability in ["director", "canvas", "content_optimization"] {
            let window = available_runtime_tools(capability, &AgentScope::default(), &[]).unwrap();
            assert!(window.schemas.len() <= MAX_AGENT_TOOL_SCHEMAS);
            assert!(
                window
                    .schemas
                    .iter()
                    .any(|schema| schema["function"]["name"] == "describe_agent_tool")
            );
            assert!(
                window
                    .schemas
                    .iter()
                    .any(|schema| schema["function"]["name"] == "load_agent_skill")
            );
        }
        let discovered = vec!["start_image_generation".into(), "delete_project".into()];
        let window =
            available_runtime_tools("director", &AgentScope::default(), &discovered).unwrap();
        assert!(
            window
                .schemas
                .iter()
                .any(|schema| schema["function"]["name"] == "start_image_generation")
        );
        assert!(
            !window
                .schemas
                .iter()
                .any(|schema| schema["function"]["name"] == "delete_project")
        );
        let restricted = AgentScope {
            allowed_tools: vec!["read_chapter_visual_plan".into()],
            ..AgentScope::default()
        };
        let window = available_runtime_tools("director", &restricted, &discovered).unwrap();
        assert!(
            !window
                .schemas
                .iter()
                .any(|schema| schema["function"]["name"] == "start_image_generation")
        );
    }
}
