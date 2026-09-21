use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use super::super::AgentLoopAction;
use crate::optimize::runtime_tools::{EDIT_ARTICLE_TOOL, GET_PROJECT_INFORMATION_TOOL};

pub(in crate::optimize::agent_loop) fn build_agent_loop_step_user_message(
    observations: &[Value],
    step: usize,
) -> Result<String, AppError> {
    let payload = json!({
        "context_refs": {"base_task": "稳定上下文包 optimization_agent_base_task"},
        "loop_state": {
            "step": step,
            "progress": agent_loop_progress(observations),
            "observations_ref": "此前 user 消息中的 loop_observation 按 sequence 顺序构成完整观察历史；保留所有成功、失败和工具结果，不能仅依据 progress 摘要决策。",
        },
    });
    build_json_task_user_message(
        "继续执行内容优化 Agent Loop，并根据当前观察选择下一步",
        "外层只输出严格 JSON 决策对象，不要输出代码块、解释文字或额外字段；可见文本字段可以使用简洁 Markdown",
        &payload,
    )
}

pub(in crate::optimize::agent_loop) fn agent_loop_rules() -> Value {
    json!({
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
    })
}

fn agent_loop_progress(observations: &[Value]) -> Value {
    let mut last_action = None;
    let mut latest_information_gaps: Option<Vec<String>> = None;
    let mut tools_used = Vec::new();
    let mut current_chapter_reads = Vec::new();
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
            let result = &observation["result"];
            current_chapter_reads.push(json!({
                "resource": result.get("resource"),
                "path": result.get("path"),
                "unit": result.get("unit"),
                "offset": result.get("offset"),
                "next_offset": result.get("next_offset"),
                "complete": result.get("complete"),
            }));
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
        "current_chapter_reads": current_chapter_reads,
        "edit_article_results": edit_article_results,
    })
}
