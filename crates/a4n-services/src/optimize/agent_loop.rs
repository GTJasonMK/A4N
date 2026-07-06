use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_engine::llm::provider::{ChatRequest, LlmTraceContext};
use a4n_engine::prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::source::source_content_sha256;

use super::runtime_tools::{EDIT_ARTICLE_TOOL, GET_PROJECT_INFORMATION_TOOL};

pub(super) const AGENT_LOOP_RUNAWAY_GUARD_STEPS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AgentLoopAction {
    Think,
    RunTool,
    Final,
}

impl AgentLoopAction {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Think => "think",
            Self::RunTool => "run_tool",
            Self::Final => "final",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct AgentLoopDecision {
    pub(super) thought: String,
    pub(super) public_update: Option<String>,
    pub(super) action: AgentLoopAction,
    pub(super) arguments: Value,
}

impl AgentLoopDecision {
    pub(super) fn public_update_for_non_final(&self) -> Result<&str, AppError> {
        self.public_update.as_deref().ok_or_else(|| {
            AppError::InvalidArgument("Agent Loop 非 final 决策缺少非空 public_update".to_string())
        })
    }
}

pub(super) fn parse_agent_loop_decision(raw: &str) -> Result<AgentLoopDecision, AppError> {
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

pub(super) fn build_agent_loop_step_user_message(
    observations: &[Value],
    step: usize,
) -> Result<String, AppError> {
    let payload = json!({
        "context_refs": {
            "base_task": "稳定上下文包 optimization_agent_base_task",
        },
        "loop_state": {
            "step": step,
            "progress": agent_loop_progress(observations),
            "observations": observations,
        },
        "completion_policy": {
            "final_is_success": "final 表示本轮任务已经完成，是正常结束动作，不是失败或放弃。",
            "return_final_when": [
                "当前用户目标已经可以被回答、保存为问题列表，或已经形成文章补丁结果。",
                "latest_information_gaps 是空数组，且已有观察足以支撑结论。",
                "edit_article 已返回 pending_confirmation，后续只需要向用户说明补丁等待审阅。",
                "工具失败后无法通过调整参数、换用工具或换用信息源继续推进，此时在 final 中说明失败点和需要用户补充的信息。",
                "缺少的信息无法由可用工具继续取得，此时在 final 中说明需要用户补充。"
            ],
            "continue_only_when": [
                "仍有具体、必要、可由工具补齐的信息缺口。",
                "下一步会产生新的观察，而不是重复已有工具调用或重复思考。",
                "最近一次工具观察 success=false，且你能通过修改参数、换用信息源或换用工具产生新的观察。",
                "该观察会直接改变最终答复、问题列表或正文补丁。"
            ],
            "do_not_continue_for": [
                "凑步骤。",
                "无理由重复相同失败工具和参数。",
                "在没有新信息缺口时继续 think。",
                "扩大到当前用户目标之外的分析。"
            ]
        },
        "protocol_constraints": {
            "root_action_enum": ["think", "run_tool", "final"],
            "tool_names_are_not_actions": [GET_PROJECT_INFORMATION_TOOL, EDIT_ARTICLE_TOOL],
            "public_update_required_for": ["think", "run_tool"],
            "public_update_purpose": "模型完成本步思考或工具决策后，主动给用户看的阶段性说明；必须说明本步公开结论或下一步行动，前端只展示该字段，不推断 Agent 意图。",
            "tool_call_shape": {
                "action": "run_tool",
                "arguments": {
                    "tool": format!("{GET_PROJECT_INFORMATION_TOOL}|{EDIT_ARTICLE_TOOL}"),
                    "arguments": "符合所选工具参数结构的对象"
                }
            },
            "invalid_examples": [
                {"action": GET_PROJECT_INFORMATION_TOOL},
                {"action": EDIT_ARTICLE_TOOL}
            ]
        },
        "response_requirement": "只返回一个严格 JSON 决策对象，不要用 Markdown 代码块包裹 JSON；public_update 和 final.reply 字符串可以使用简洁 Markdown 提高可读性。",
    });
    build_json_task_user_message(
        "继续执行内容优化 Agent Loop，并根据当前观察选择下一步",
        "外层只输出严格 JSON 决策对象，不要输出代码块、解释文字或额外字段；可见文本字段可以使用简洁 Markdown",
        &payload,
    )
}

pub(super) fn build_first_agent_loop_chat_request(
    project_id: i64,
    session_id: Option<i64>,
    workflow: &str,
    system_prompt: String,
    base_user_message: &str,
) -> Result<ChatRequest, AppError> {
    build_agent_loop_chat_request(
        project_id,
        session_id,
        workflow,
        system_prompt,
        base_user_message,
        &[],
        1,
    )
}

pub(super) fn build_agent_loop_chat_request(
    project_id: i64,
    session_id: Option<i64>,
    workflow: &str,
    system_prompt: String,
    base_user_message: &str,
    observations: &[Value],
    step: usize,
) -> Result<ChatRequest, AppError> {
    let step_user_message = build_agent_loop_step_user_message(observations, step)?;
    let manifest = build_agent_loop_context_manifest(base_user_message, step_user_message)?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.2),
        None,
        Some(llm_token_profiles::JSON_STANDARD.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: workflow.to_string(),
        step_key: session_id
            .map(|id| format!("session_{id}_step_{step}"))
            .unwrap_or_else(|| format!("preview_step_{step}")),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(
            session_id
                .map(|id| format!("session_{id}_step_{step}"))
                .unwrap_or_else(|| format!("preview_step_{step}")),
        ),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn build_agent_loop_context_manifest(
    base_user_message: &str,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let base_user_message = base_user_message.trim();
    if base_user_message.is_empty() {
        return Err(AppError::InvalidArgument(
            "Agent Loop 缺少基础任务输入".to_string(),
        ));
    }
    let mut ledger = LlmContextLedger::new();
    let base_hash = source_content_sha256(base_user_message);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("optimization_session").into(),
        pack_kind: ("optimization_agent_base_task").into(),
        source_refs_json: Some(json!({
            "source": "optimization_agent_base_task",
            "content_sha256": base_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "optimization_agent_loop_context_v1",
            "kind": "base_task",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "base_task",
                    "base_task": base_user_message,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "base_task",
                    "base_task": base_user_message,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "base_task",
                    "base_task": base_user_message,
                }),
            ),
        ],
    })?;
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "Agent Loop 每步必须只返回严格 JSON 决策对象；action 只能是 think、run_tool 或 final。"
                .to_string(),
        ),
    ))
}

pub(super) fn agent_loop_request_payload(request: &ChatRequest) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(
        &context_aware_chat_request_trace_json(request)?,
    )?)
}

fn agent_loop_progress(observations: &[Value]) -> Value {
    let mut last_action = None;
    let mut latest_information_gaps: Option<Vec<String>> = None;
    let mut tools_used = Vec::new();
    let mut current_chapter_read = false;
    let mut edit_article_results = Vec::new();

    for observation in observations {
        let action = observation
            .get("action")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        if action.is_some() {
            last_action = action.clone();
        }

        if action.as_deref() == Some(AgentLoopAction::Think.as_str()) {
            latest_information_gaps = observation
                .get("information_gaps")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                });
        }

        if action.as_deref() != Some(AgentLoopAction::RunTool.as_str()) {
            continue;
        }

        let tool = observation
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let arguments = observation
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let success = observation
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if tool == GET_PROJECT_INFORMATION_TOOL
            && arguments.get("mode").and_then(Value::as_str) == Some("read")
            && arguments.get("source").and_then(Value::as_str) == Some("current_chapter")
            && success
        {
            current_chapter_read = true;
        }

        if tool == EDIT_ARTICLE_TOOL {
            let result = observation
                .get("result")
                .cloned()
                .unwrap_or_else(|| json!({}));
            edit_article_results.push(json!({
                "mode": result.get("mode").cloned().unwrap_or(Value::Null),
                "patch_count": result.get("patch_count").cloned().unwrap_or(Value::Null),
                "requires_user_confirmation": result
                    .get("requires_user_confirmation")
                    .cloned()
                    .unwrap_or(Value::Null),
            }));
        }

        tools_used.push(json!({
            "tool": tool,
            "arguments": arguments,
            "success": success,
        }));
    }

    json!({
        "observation_count": observations.len(),
        "last_action": last_action,
        "latest_information_gaps": latest_information_gaps,
        "tools_used": tools_used,
        "current_chapter_read": current_chapter_read,
        "edit_article_results": edit_article_results,
    })
}

#[derive(Debug)]
pub(super) struct ThinkingObservation {
    pub(super) value: Value,
    pub(super) event_summary: String,
}

pub(super) fn thinking_observation(
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

pub(super) fn insert_current_chapter_content(context: &mut Value, content: &str) {
    if let Some(current_chapter) = context
        .get_mut("current_chapter")
        .and_then(Value::as_object_mut)
    {
        current_chapter.insert("content".to_string(), json!(content));
    }
}

pub(super) fn run_tool_arguments(arguments: &Value) -> Result<(&str, Value), AppError> {
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

pub(super) fn final_issues(arguments: &Value) -> Result<Value, AppError> {
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

pub(super) fn final_reply(arguments: &Value) -> Result<String, AppError> {
    reject_unknown_action_argument_fields(arguments, &["reply"], "final")?;
    arguments
        .get("reply")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument("final 缺少非空 reply".to_string()))
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

fn reject_unknown_action_argument_fields(
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

pub(super) fn tool_observation(
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

pub(super) fn failed_tool_observation(
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

pub(super) fn final_response_from_issues(issues: Value) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(&json!({
        "issues": issues,
    }))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_loop_decision() {
        let decision = parse_agent_loop_decision(
            r#"{"thought":"先读正文","public_update":"我会先读取当前章节，确认需要处理的内容。","action":"run_tool","arguments":{"tool":"get_project_information","arguments":{"mode":"read","source":"current_chapter"}}}"#,
        )
        .unwrap();

        assert_eq!(decision.thought, "先读正文");
        assert_eq!(
            decision.public_update.as_deref(),
            Some("我会先读取当前章节，确认需要处理的内容。")
        );
        assert_eq!(decision.action, AgentLoopAction::RunTool);
        assert_eq!(decision.arguments["tool"], "get_project_information");
    }

    #[test]
    fn parses_thinking_observation() {
        let decision = parse_agent_loop_decision(
            r#"{"thought":"先明确目标","public_update":"我需要先确认当前请求是否依赖章节正文。","action":"think","arguments":{"focus":"判断当前请求是否需要上下文","known_facts":["用户要求优化当前内容"],"information_gaps":["需要当前章节正文"],"next_step":"读取 current_chapter"}}"#,
        )
        .unwrap();
        let observation = thinking_observation(1, &decision.thought, &decision.arguments).unwrap();

        assert_eq!(decision.action, AgentLoopAction::Think);
        assert_eq!(observation.value["action"], "think");
        assert!(
            observation
                .event_summary
                .contains("判断当前请求是否需要上下文")
        );
    }

    #[test]
    fn rejects_invalid_thinking_arguments() {
        let decision = parse_agent_loop_decision(
            r#"{"thought":"x","public_update":"我已经完成本步判断。","action":"think","arguments":{"focus":"x","known_facts":[],"information_gaps":[]}}"#,
        )
        .unwrap();
        let err = thinking_observation(1, &decision.thought, &decision.arguments).unwrap_err();

        assert!(err.to_string().contains("next_step"));
    }

    #[test]
    fn rejects_non_json_decision() {
        let err = parse_agent_loop_decision("```json\n{}\n```").unwrap_err();

        assert!(err.to_string().contains("严格 JSON 决策对象"));
    }

    #[test]
    fn rejects_unknown_action() {
        let err =
            parse_agent_loop_decision(
                r#"{"thought":"x","public_update":"准备修改正文。","action":"edit_article","arguments":{}}"#,
            )
            .unwrap_err();

        assert!(err.to_string().contains("action 非法"));
    }

    #[test]
    fn rejects_unknown_decision_field() {
        let err = parse_agent_loop_decision(
            r#"{"thought":"x","public_update":"我会继续。","action":"think","arguments":{"focus":"x","known_facts":[],"information_gaps":[],"next_step":"结束"},"legacy":true}"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("未知字段 legacy"));
    }

    #[test]
    fn rejects_non_final_decision_without_public_update() {
        let err = parse_agent_loop_decision(r#"{"thought":"x","action":"think","arguments":{}}"#)
            .unwrap_err();

        assert!(err.to_string().contains("public_update"));
    }

    #[test]
    fn rejects_unknown_think_argument_field() {
        let decision = parse_agent_loop_decision(
            r#"{"thought":"x","public_update":"我会先整理判断。","action":"think","arguments":{"focus":"x","known_facts":[],"information_gaps":[],"next_step":"结束","extra":true}}"#,
        )
        .unwrap();
        let err = thinking_observation(1, &decision.thought, &decision.arguments).unwrap_err();

        assert!(err.to_string().contains("未知字段 extra"));
    }

    #[test]
    fn step_user_message_keeps_tool_names_out_of_root_action() {
        let message = build_agent_loop_step_user_message(&[], 1).unwrap();

        assert!(message.contains("protocol_constraints"));
        assert!(message.contains("tool_names_are_not_actions"));
        assert!(message.contains("optimization_agent_base_task"));
        assert!(message.contains("public_update_required_for"));
        assert!(message.contains(r#""action": "edit_article""#));
        assert!(message.contains(r#""action": "run_tool""#));
    }

    #[test]
    fn agent_loop_request_moves_base_task_into_context_manifest() {
        let request = build_first_agent_loop_chat_request(
            42,
            Some(7),
            "content_optimization_agent_loop",
            "系统提示词".to_string(),
            "优化当前章节正文：主角突然转向。",
        )
        .unwrap();

        let manifest = request
            .context_manifest_json
            .as_ref()
            .expect("context manifest should be attached");
        let pack_kinds = manifest["packs"]
            .as_array()
            .expect("packs should be array")
            .iter()
            .filter_map(|pack| pack["pack_kind"].as_str())
            .collect::<Vec<_>>();
        assert!(pack_kinds.contains(&"optimization_agent_base_task"));

        let user_message = request
            .messages
            .iter()
            .find(|message| matches!(message.role, a4n_engine::llm::provider::MessageRole::User))
            .expect("user message should exist");
        assert!(user_message.content.contains("context_refs"));
        assert!(
            user_message
                .content
                .contains("optimization_agent_base_task")
        );
        assert!(!user_message.content.contains("主角突然转向"));

        let trace_context = request
            .trace_context
            .as_ref()
            .expect("trace context should be attached");
        assert_eq!(trace_context.workflow, "content_optimization_agent_loop");
        assert_eq!(trace_context.call_key.as_deref(), Some("session_7_step_1"));
    }
}
