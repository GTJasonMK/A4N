use a4n_api_contract::agent::{
    AgentMemoryScopeKind, AgentScope, AgentSkillDescriptor, ReviewAgentDelegationInput,
};
use a4n_domain::agent::{AGENT_PROTOCOL_JSON_COMPAT, AGENT_PROTOCOL_NATIVE_TOOL_CALLING};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{
    ChatRequest, ChatResponse, ModelCapabilities, ResponseFormat, ToolCall,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLlmCapabilities {
    pub model: ModelCapabilities,
    pub agent_protocol: String,
}

pub const AGENT_WORKER_ID: &str = "agent-worker";
pub const AGENT_FAILURE_PROVIDER_UNAVAILABLE: &str = "provider_unavailable";
pub const AGENT_THREAD_ENSURE_MAIN_OPERATION: &str = "agent.thread.ensure_main";
pub const AGENT_THREAD_CREATE_OPERATION: &str = "agent.thread.create";
pub const AGENT_THREAD_FORK_OPERATION: &str = "agent.thread.fork";
pub const AGENT_THREAD_UPDATE_OPERATION: &str = "agent.thread.update";
pub const AGENT_THREAD_ARCHIVE_OPERATION: &str = "agent.thread.archive";
pub const AGENT_THREAD_LIST_OPERATION: &str = "agent.thread.list";
pub const AGENT_THREAD_SHOW_OPERATION: &str = "agent.thread.show";
pub const AGENT_THREAD_MESSAGES_OPERATION: &str = "agent.thread.messages";
pub const AGENT_RUN_SHOW_OPERATION: &str = "agent.run.show";
pub const AGENT_EVENT_LIST_OPERATION: &str = "agent.event.list";
pub const AGENT_MEMORY_LIST_OPERATION: &str = "agent.memory.list";
pub const AGENT_MEMORY_CREATE_OPERATION: &str = "agent.memory.create";
pub const AGENT_MEMORY_RETRACT_OPERATION: &str = "agent.memory.retract";
pub const AGENT_APPROVAL_RESOLVE_OPERATION: &str = "agent.approval.resolve";
pub const AGENT_DELEGATION_SPAWN_OPERATION: &str = "agent.delegation.spawn";
pub const AGENT_DELEGATION_REVIEW_OPERATION: &str = "agent.delegation.review";
pub const AGENT_SKILL_SETTINGS_LIST_OPERATION: &str = "agent.skill.settings.list";
pub const AGENT_SKILL_SETTINGS_UPDATE_OPERATION: &str = "agent.skill.settings.update";
pub const AGENT_SKILL_SEARCH_OPERATION: &str = "agent.skill.search";
pub const AGENT_SKILL_LOAD_OPERATION: &str = "agent.skill.load";
pub const AGENT_TOOL_SEARCH_OPERATION: &str = "agent.tool.search";
pub const AGENT_TOOL_DESCRIBE_OPERATION: &str = "agent.tool.describe";
pub const AGENT_RUN_SEND_OPERATION: &str = "agent.run.send";
pub const AGENT_RUN_CONTROL_OPERATION: &str = "agent.run.control";
pub const AGENT_RUN_SEND_CHILD_OPERATION: &str = "agent.run.send_child";
pub const AGENT_RUN_CONTROL_CHILD_OPERATION: &str = "agent.run.control_child";

pub fn provider_failure_class(error: &AppError) -> Option<&'static str> {
    let detail = error.to_string().to_ascii_lowercase();
    (detail.contains("503")
        || detail.contains("no healthy upstream")
        || detail.contains("service unavailable")
        || detail.contains("provider unavailable"))
    .then_some(AGENT_FAILURE_PROVIDER_UNAVAILABLE)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentMemoryListInput {
    pub scope_kind: Option<AgentMemoryScopeKind>,
    pub scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentDelegationReviewCommand {
    pub input: ReviewAgentDelegationInput,
    pub parent_run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillCapabilityCommand<T> {
    pub capability: String,
    pub input: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentRunRuntimeCommand<T> {
    pub input: T,
    pub runtime_snapshot_required: bool,
    pub require_runtime_snapshot_for_new_run: bool,
}

pub fn agent_skill_directory_prompt(skills: &[AgentSkillDescriptor]) -> String {
    if skills.is_empty() {
        return "当前 capability 没有启用 Skill。".into();
    }
    skills
        .iter()
        .map(|skill| format!("- {} ({})：{}", skill.id, skill.version, skill.description))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn ensure_agent_scope_allows_tool(scope: &AgentScope, name: &str) -> Result<(), AppError> {
    if a4n_api_contract::agent::is_agent_skill_tool(name)
        || scope.allowed_tools.is_empty()
        || scope.allowed_tools.iter().any(|allowed| allowed == name)
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "Agent scope 不允许使用工具: {name}"
        )))
    }
}

/// Serialize the trusted Agent worker-to-Control request with the internal
/// context needed by the Control LLM runtime. `ChatRequest` still omits this
/// context from provider wire serialization.
pub fn model_request_wire_value(request: &ChatRequest) -> Result<Value, AppError> {
    request.snapshot_value()
}

pub fn restore_model_request_wire(value: Value) -> Result<ChatRequest, AppError> {
    ChatRequest::from_snapshot_value(value)
}

#[derive(Debug, Clone)]
pub enum AgentModelDecision {
    Final {
        reply: String,
    },
    Think {
        content: String,
    },
    ToolCalls {
        content: String,
        calls: Vec<ToolCall>,
    },
}

pub fn configure_agent_request(
    request: &mut ChatRequest,
    protocol: &str,
    tools: Vec<Value>,
) -> Result<(), AppError> {
    if protocol == AGENT_PROTOCOL_JSON_COMPAT {
        request.tools = None;
        request.tool_choice = None;
        request.response_format = Some(ResponseFormat::json_object());
        request.protect_prompt_output_contract(
            "agent_json_compat",
            agent_json_compat_output_schema(),
        )?;
    } else {
        request.tools = (!tools.is_empty()).then_some(tools);
        request.tool_choice = None;
        request.response_format = None;
    }
    Ok(())
}

pub fn agent_protocol_prompt(protocol: &str, tools: &[Value]) -> String {
    const TOOL_SEARCH_RULES: &str = "search_agent_tools 规则：query 使用空格分隔的关键词；domain 填 manifest domain（如 optimize、canvas、project、task），不要填 Agent capability；effect 只能是 read、write 或 destructive；受限 capability Agent 省略 capability 字段，运行时会自动限制到当前 capability。";
    if protocol != AGENT_PROTOCOL_JSON_COMPAT {
        return format!(
            "模型协议：使用服务商原生 Tool Calling。工具名称和参数必须通过原生工具调用发送，不要把工具名写成普通文本。{TOOL_SEARCH_RULES}"
        );
    }
    format!(
        "模型协议：当前 LLM 未启用原生 Tool Calling。每次只能输出一个严格 JSON 对象，不要 Markdown、代码围栏或解释。\n\
         JSON 根对象只能包含 action 和 arguments；action 只能是 think、run_tool、final。\n\
         think 的 arguments 为 {{\"thought\": string, \"public_update\": string}}；\n\
         run_tool 的 arguments 为 {{\"tool\": string, \"arguments\": object}}；\n\
         final 的 arguments 为 {{\"reply\": string}}。\n\
         {TOOL_SEARCH_RULES}\n\
         可用工具定义：{}",
        serde_json::to_string(tools).expect("Agent tool schemas must serialize")
    )
}

pub fn decode_agent_response(
    response: &ChatResponse,
    protocol: &str,
    execution_id: i64,
) -> Result<AgentModelDecision, AppError> {
    a4n_llm_provider_contract::validate_finish_reason(response.finish_reason.as_deref())?;
    if protocol != AGENT_PROTOCOL_JSON_COMPAT {
        let calls = response.message.tool_calls.clone().unwrap_or_default();
        if calls.is_empty() {
            let reply = response.message.content.trim();
            if reply.is_empty() {
                return Err(AppError::LlmError(
                    "Agent model returned empty final content".into(),
                ));
            }
            return Ok(AgentModelDecision::Final {
                reply: reply.to_string(),
            });
        }
        let mut ids = std::collections::HashSet::new();
        for call in &calls {
            if call.id.trim().is_empty()
                || call.name.trim().is_empty()
                || !call.arguments.is_object()
                || !ids.insert(call.id.as_str())
            {
                return Err(AppError::LlmError("Agent model tool calls require unique nonempty ids, a tool name and object arguments".into()));
            }
        }
        return Ok(AgentModelDecision::ToolCalls {
            content: response.message.content.clone(),
            calls,
        });
    }

    if response
        .message
        .tool_calls
        .as_ref()
        .is_some_and(|calls| !calls.is_empty())
    {
        return Err(AppError::LlmError(
            "Agent JSON 协议不得混入原生工具调用".into(),
        ));
    }
    let value = a4n_domain::json_contract::parse_strict_json_value(response.message.content.trim())
        .map_err(|error| {
            AppError::LlmError(format!("Agent json_compat 响应不是严格 JSON 对象: {error}"))
        })?;
    parse_json_compat_decision(value, execution_id)
}

pub fn agent_protocol_from_request_snapshot(value: &Value) -> &str {
    value
        .get("response_format")
        .and_then(|format| format.get("type"))
        .and_then(Value::as_str)
        .filter(|kind| *kind == "json_object")
        .map(|_| AGENT_PROTOCOL_JSON_COMPAT)
        .unwrap_or(AGENT_PROTOCOL_NATIVE_TOOL_CALLING)
}

fn parse_json_compat_decision(
    value: Value,
    execution_id: i64,
) -> Result<AgentModelDecision, AppError> {
    a4n_domain::json_contract::OutputSchemaValidator::new(&agent_json_compat_output_schema())
        .map_err(AppError::ConfigError)?
        .validate(&value)
        .map_err(AppError::InvalidArgument)?;
    let content = serde_json::to_string(&value)?;
    match serde_json::from_value::<JsonCompatDecision>(value).map_err(AppError::Serialization)? {
        JsonCompatDecision::Think {
            thought,
            public_update,
        } => Ok(AgentModelDecision::Think {
            content: serde_json::to_string(&json!({
                "action": "think",
                "arguments": {"thought": thought.trim(), "public_update": public_update.trim()},
            }))?,
        }),
        JsonCompatDecision::RunTool { tool, arguments } => Ok(AgentModelDecision::ToolCalls {
            content,
            calls: vec![ToolCall {
                id: format!("compat-tool-{execution_id}"),
                name: tool.trim().to_string(),
                arguments: Value::Object(arguments),
                stream_index: None,
                provider_metadata: None,
            }],
        }),
        JsonCompatDecision::Final { reply } => Ok(AgentModelDecision::Final {
            reply: reply.trim().to_string(),
        }),
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(
    tag = "action",
    content = "arguments",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum JsonCompatDecision {
    Think {
        #[schemars(length(min = 1), regex(pattern = r"\S"))]
        thought: String,
        #[schemars(length(min = 1), regex(pattern = r"\S"))]
        public_update: String,
    },
    RunTool {
        #[schemars(length(min = 1), regex(pattern = r"\S"))]
        tool: String,
        arguments: Map<String, Value>,
    },
    Final {
        #[schemars(length(min = 1), regex(pattern = r"\S"))]
        reply: String,
    },
}

pub fn agent_json_compat_output_schema() -> Value {
    a4n_domain::json_contract::typed_output_schema::<JsonCompatDecision>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm_provider_contract::provider::{ChatMessage, ChatResponse, MessageRole};

    #[test]
    fn model_contract_matrix_agent_json_schema() {
        a4n_test_support::model_contract::emit_schema(
            "agent_json_compat",
            agent_json_compat_output_schema(),
        );
    }

    #[test]
    #[ignore = "model runner supplies Agent JSON decision vectors"]
    fn model_contract_matrix_agent_json_boundary() {
        let document: Value = serde_json::from_slice(
            &std::fs::read(std::env::var("A4N_MODEL_CONTRACT_CASES").unwrap()).unwrap(),
        )
        .unwrap();
        for case in document["cases"].as_array().unwrap() {
            let actual = decode_agent_response(
                &response(&case["value"].to_string()),
                AGENT_PROTOCOL_JSON_COMPAT,
                7,
            );
            a4n_test_support::model_contract::assert_case(
                "agent_json_compat",
                "runtime-compat",
                case["vector_key"].as_str().unwrap(),
                "validated-output",
                case["expected"].clone(),
                json!(actual.is_ok()),
                json!({"decoder":"decode_agent_response"}),
            );
        }
    }

    fn response(content: &str) -> ChatResponse {
        ChatResponse {
            message: ChatMessage::new(MessageRole::Assistant, content),
            finish_reason: Some("stop".into()),
            usage: None,
            reasoning_content: None,
        }
    }

    #[test]
    fn json_compat_decodes_tool_and_final() {
        let tool = decode_agent_response(
            &response(r#"{"action":"run_tool","arguments":{"tool":"read_project_context","arguments":{}}}"#),
            AGENT_PROTOCOL_JSON_COMPAT,
            42,
        )
        .expect("tool decision");
        assert!(
            matches!(tool, AgentModelDecision::ToolCalls { calls, .. } if calls[0].id == "compat-tool-42")
        );
        let final_decision = decode_agent_response(
            &response(r#"{"action":"final","arguments":{"reply":"完成"}}"#),
            AGENT_PROTOCOL_JSON_COMPAT,
            42,
        )
        .expect("final decision");
        assert!(matches!(
            final_decision,
            AgentModelDecision::Final { reply } if reply == "完成"
        ));
    }

    #[test]
    fn json_compat_rejects_markdown_and_unknown_fields() {
        assert!(
            decode_agent_response(&response("```json{}"), AGENT_PROTOCOL_JSON_COMPAT, 1).is_err()
        );
        assert!(
            decode_agent_response(
                &response(r#"{"action":"final","arguments":{"reply":"ok"},"legacy":true}"#),
                AGENT_PROTOCOL_JSON_COMPAT,
                1,
            )
            .is_err()
        );
    }

    #[test]
    fn replay_protocol_is_derived_from_the_persisted_request_snapshot() {
        let mut request = ChatRequest::system_user("system", "task", None, None);
        configure_agent_request(&mut request, AGENT_PROTOCOL_JSON_COMPAT, Vec::new()).unwrap();
        let snapshot = request.snapshot_value().expect("request snapshot");

        assert_eq!(
            agent_protocol_from_request_snapshot(&snapshot),
            AGENT_PROTOCOL_JSON_COMPAT
        );
    }

    #[test]
    fn completed_model_response_must_be_valid_before_replay() {
        let mut value = response("完成");
        value.finish_reason = Some("length".into());
        assert!(decode_agent_response(&value, AGENT_PROTOCOL_NATIVE_TOOL_CALLING, 1).is_err());
        value.finish_reason = Some("tool_calls".into());
        let call = ToolCall {
            id: "call-1".into(),
            name: "read_project_context".into(),
            arguments: json!({}),
            stream_index: None,
            provider_metadata: None,
        };
        value.message.tool_calls = Some(vec![call.clone()]);
        assert!(decode_agent_response(&value, AGENT_PROTOCOL_NATIVE_TOOL_CALLING, 1).is_ok());
        value.message.tool_calls = Some(vec![call.clone(), call]);
        assert!(decode_agent_response(&value, AGENT_PROTOCOL_NATIVE_TOOL_CALLING, 1).is_err());
        value.message.tool_calls.as_mut().unwrap().truncate(1);
        value.message.tool_calls.as_mut().unwrap()[0].arguments = json!([]);
        assert!(decode_agent_response(&value, AGENT_PROTOCOL_NATIVE_TOOL_CALLING, 1).is_err());
    }
}
