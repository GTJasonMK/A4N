use super::*;
use std::sync::atomic::AtomicI64;

pub(crate) struct RuntimeToolWindow {
    pub(crate) schemas: Vec<Value>,
    pub(crate) dropped_optional: Vec<String>,
}

pub(crate) fn available_runtime_tools(
    capability: &str,
    scope: &AgentScope,
    phase: &str,
    discovered_tools: &[String],
    executions: &[AgentExecutionSnapshot],
) -> Result<RuntimeToolWindow, AppError> {
    let initial_tools = a4n_api_contract::agent::initial_agent_tools(capability);
    let restricted = a4n_api_contract::agent::capability_has_restricted_tool_catalog(capability);
    let initial_limit = if restricted {
        usize::MAX
    } else {
        MAX_BASE_AGENT_TOOLS
    };
    let mut names = initial_tools
        .iter()
        .map(|name| (*name).to_string())
        .take(initial_limit)
        .collect::<Vec<_>>();
    for tool in ["read_image", "web_search", "web_fetch"] {
        if initial_tools.contains(&tool) && !names.iter().any(|name| name == tool) {
            names.push(tool.into());
        }
    }
    names.retain(|name| {
        ensure_agent_scope_allows_tool(scope, name).is_ok()
            && a4n_api_contract::agent::capability_allows_tool(capability, name)
            && a4n_api_contract::agent::agent_phase_allows_tool(phase, name)
    });
    let pending = executions
        .iter()
        .filter(|execution| {
            matches!(
                execution.status.as_str(),
                "prepared" | "awaiting_user" | "dispatching" | "running"
            )
        })
        .filter_map(|execution| execution.tool_name.as_deref())
        .filter(|name| ensure_agent_scope_allows_tool(scope, name).is_ok())
        .collect::<Vec<_>>();
    for name in pending.iter().copied() {
        if !names.iter().any(|known| known == name) {
            names.push(name.to_string());
        }
    }
    if names.len() > MAX_AGENT_TOOL_SCHEMAS {
        return Err(AppError::InvalidState {
            current: format!(
                "Agent capability {capability} 在 phase {phase} 需要 {} 个 mandatory 工具，但单次请求最多保留 {} 个工具 schema: {}",
                names.len(),
                MAX_AGENT_TOOL_SCHEMAS,
                names.join(", ")
            ),
            operation: "构建 Agent 工具窗口".into(),
        });
    }
    if restricted {
        return names
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
                dropped_optional: Vec::new(),
            });
    }
    let mut dropped_optional = Vec::new();
    for name in discovered_tools.iter().rev() {
        if !names.iter().any(|known| known == name)
            && ensure_agent_scope_allows_tool(scope, name).is_ok()
        {
            if names.len() == MAX_AGENT_TOOL_SCHEMAS {
                dropped_optional.push(name.clone());
                continue;
            }
            names.push(name.clone());
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
    message: String,
    messages: &mut Vec<ChatMessage>,
    latest_message_id: &AtomicI64,
) -> Result<(RunProgress, Vec<String>), AppError> {
    execution_application
        .update_worker_execution(WorkerExecutionUpdate {
            run_id: claimed.run.id,
            lease_token: claimed.lease_token.clone(),
            execution_id: execution.id,
            status: "failed".into(),
            result_json: None,
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
        json!({"error": message}),
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
    if result.get("error").is_some() {
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
    is_discovery_tool(name)
        .then(|| serde_json::to_string(arguments).ok())
        .flatten()
        .map(|arguments| format!("{name}:{arguments}"))
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

pub(crate) fn json_u32(value: &Value, key: &str) -> Result<u32, AppError> {
    u32::try_from(json_u64(value, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run".into(),
    })
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
