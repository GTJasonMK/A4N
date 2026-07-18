use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_session, optimization_turn};
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use crate::optimize::{
    OPTIMIZATION_CHAT_CONTENT_LIMIT, OPTIMIZATION_CHAT_HISTORY_LIMIT, OptimizeAgentPermissions,
};

pub(in crate::optimize) fn build_content_optimization_chat_user_message(
    user_message: &str,
    session: &optimization_session::Model,
    conversation_history: &Value,
    context_manifest: &Value,
) -> Result<String, AppError> {
    let user_message = user_message.trim();
    if user_message.is_empty() {
        return Err(AppError::InvalidArgument("对话消息不能为空".to_string()));
    }

    let payload = json!({
        "write_policy": {
            "automatic_writeback": OptimizeAgentPermissions::from_session(session).automatic_writeback,
            "edit_article_result": "edit_article 只返回待审阅补丁；automatic_writeback=false 时逐条审阅，automatic_writeback=true 时批量审阅",
        },
        "interaction_policy": {
            "primary_goal": "围绕当前 user_message 回答用户真实问题",
            "direct_final": "如果当前消息本身已经足以回答，直接 final.reply，不获取上下文",
            "context_access": "如果当前用户问题明确需要章节正文、段落、项目设定或已有问题列表，可以调用 get_project_information 获取必要上下文",
            "write_boundary": "调用 edit_article 不会直接写回正文；真正写回只发生在用户接受并同步补丁之后",
            "analysis_boundary": "不得扩大成完整章节分析；只围绕当前 user_message 读取必要上下文和给出回答",
        },
        "completion_policy": {
            "final_is_success": "final 是当前用户消息已处理完成的正常动作，不是失败或放弃。",
            "finish_when": [
                "已有信息足以回答 user_message。",
                "edit_article 已返回 pending_confirmation，需要向用户说明补丁等待审阅。",
                "工具失败后无法通过调整参数、换用工具或换用信息源继续推进，需要向用户说明失败点和可补充的信息。",
                "缺失信息无法由可用工具补齐，需要让用户补充。"
            ],
            "continue_only_when": [
                "仍有明确、必要、可由工具补齐的信息缺口。",
                "最近一次工具观察 success=false，且可以通过修改参数、换用信息源或换用工具产生新观察。",
                "下一步会产生新的观察并影响最终 reply 或正文补丁。"
            ],
        },
        "available_actions": {
            "think": {
                "arguments": {
                    "focus": "本步要解决的判断焦点",
                    "known_facts": "字符串数组，已经从输入或观察中确认的事实",
                    "information_gaps": "字符串数组，回答用户当前问题仍缺少的信息",
                    "next_step": "下一步准备执行的动作意图"
                },
                "purpose": "复杂请求先形成可展示、可审计的计划或反思观察；当前消息本身足以回答时不要调用"
            },
            "run_tool": {
                "arguments": {
                    "tool": "已注册优化工具名",
                    "arguments": "符合该工具参数结构的对象"
                },
                "purpose": "调用信息检索或文章补丁工具；不会直接写回正文"
            },
            "final": {
                "arguments": {"reply": "直接给用户看的自然语言回复"},
                "purpose": "结束本轮对话"
            }
        },
        "protocol_constraints": {
            "root_action_enum": ["think", "run_tool", "final"],
            "tool_names_are_not_actions": ["get_project_information", "edit_article"],
            "thought_visibility": "thought 是内部工作摘要，不进入用户界面；不要把给用户看的话放在 thought。",
            "public_update_required_for": ["think", "run_tool"],
            "public_update_purpose": "模型完成本步思考或工具决策后，主动给用户看的阶段性说明；必须说明本步公开结论或下一步行动，前端只展示该字段，不推断 Agent 意图。",
            "tool_call_shape": {
                "action": "run_tool",
                "arguments": {
                    "tool": "get_project_information|edit_article",
                    "arguments": "符合所选工具参数结构的对象"
                }
            },
            "invalid_examples": [
                {"action": "get_project_information"},
                {"action": "edit_article"}
            ]
        },
        "session": {
            "id": session.id,
            "status": session.status.as_str(),
            "issues": session.issues.clone(),
            "summary": session.summary.as_deref(),
        },
        "context_manifest": context_manifest,
        "conversation_history": conversation_history,
        "user_message": user_message,
        "task": {
            "name": "content_optimization_chat_agent_loop",
            "requirement": "以当前 user_message 为中心自主决定下一步：直接回复、规划、通过工具获取必要信息、提出文章补丁观察，或输出 final.reply；不得写回数据库。",
        },
        "decision_contract": {
            "root_type": "JSON 对象",
            "required_fields": {
                "thought": "字符串，内部工作摘要，不展示给用户，不展开隐藏推理链",
                "public_update": "字符串，think/run_tool 必填，给用户看的阶段性说明；可使用简洁 Markdown",
                "action": "think|run_tool|final",
                "arguments": "对象"
            },
            "think_arguments": {
                "focus": "非空字符串",
                "known_facts": "字符串数组",
                "information_gaps": "字符串数组",
                "next_step": "非空字符串"
            },
            "final_arguments": {"reply": "非空字符串"}
        },
    });
    build_json_task_user_message(
        "继续内容优化对话 Agent Loop",
        "每次只返回 decision_contract 要求的一个严格 JSON 决策对象，不要输出代码块、解释文字或额外字段；public_update 和 final.reply 字符串可以使用简洁 Markdown",
        &payload,
    )
}

pub(in crate::optimize) fn optimization_chat_history(turns: &[optimization_turn::Model]) -> Value {
    let mut recent = turns
        .iter()
        .rev()
        .take(OPTIMIZATION_CHAT_HISTORY_LIMIT)
        .collect::<Vec<_>>();
    recent.reverse();

    Value::Array(
        recent
            .into_iter()
            .map(|turn| {
                let (user_text, user_truncated, user_original_chars) =
                    truncate_chat_content(&turn.user_text, OPTIMIZATION_CHAT_CONTENT_LIMIT);
                let final_reply = turn.final_reply.as_deref().map(|reply| {
                    let (content, truncated, original_chars) =
                        truncate_chat_content(reply, OPTIMIZATION_CHAT_CONTENT_LIMIT);
                    json!({
                        "content": content,
                        "truncated": truncated,
                        "original_chars": original_chars,
                    })
                });
                json!({
                    "turn_id": turn.id,
                    "status": turn.status,
                    "user": {
                        "content": user_text,
                        "truncated": user_truncated,
                        "original_chars": user_original_chars,
                    },
                    "assistant": final_reply,
                    "created_at": turn.created_at.to_string(),
                    "updated_at": turn.updated_at.to_string(),
                })
            })
            .collect(),
    )
}

fn truncate_chat_content(content: &str, limit: usize) -> (String, bool, usize) {
    let original_chars = content.chars().count();
    if original_chars <= limit {
        return (content.to_string(), false, original_chars);
    }
    let mut truncated = content.chars().take(limit).collect::<String>();
    truncated.push_str(&format!("\n...[内容已截断，原始字符数 {original_chars}]"));
    (truncated, true, original_chars)
}
