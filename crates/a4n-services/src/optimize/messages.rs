use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_session, optimization_turn};
use a4n_engine::prompt::request::{build_json_task_user_message, json_user_message_payload};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::permissions::optimization_agent_capabilities;
use super::{
    OPTIMIZATION_CHAT_CONTENT_LIMIT, OPTIMIZATION_CHAT_HISTORY_LIMIT, OPTIMIZATION_PATCH_MAX_COUNT,
    OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS, OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
    OptimizeAgentPermissions,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct OptimizeIssue {
    pub id: String,
    pub tool: String,
    pub arguments: Value,
    pub description: String,
    pub severity: String,
}

pub(super) fn build_content_optimization_user_message(
    chapter_content: &str,
    context_manifest: Option<&Value>,
    permissions: &OptimizeAgentPermissions,
    agent_instruction: Option<&str>,
) -> Result<String, AppError> {
    let chapter_content = chapter_content.trim();
    if chapter_content.is_empty() {
        return Err(AppError::InvalidArgument(
            "内容优化分析缺少章节正文".to_string(),
        ));
    }
    let payload = json!({
        "agent": {
            "permissions": permissions.as_value(),
            "capabilities": optimization_agent_capabilities(permissions),
            "user_instruction": agent_instruction,
            "interaction_model": "显式 Agent Loop：先计划，再通过 run_tool 调用信息检索或补丁工具获取观察，最后用 final 输出 issues。",
        },
        "completion_policy": {
            "final_is_success": "final 是任务完成的正常动作，不是失败或放弃。",
            "finish_when": [
                "已读取 current_chapter，且已有观察足以判断问题是否存在。",
                "没有确实存在的问题，输出空 issues。",
                "工具失败后无法通过调整参数、换用工具或换用信息源继续推进，基于已知事实输出结果并说明失败点。",
                "缺失信息无法由可用工具补齐，基于已知事实输出结果。"
            ],
            "continue_only_when": [
                "仍有明确、必要、可由工具补齐的信息缺口。",
                "最近一次工具观察 success=false，且可以通过修改参数、换用信息源或换用工具产生新观察。",
                "下一步会产生新的观察并影响最终 issues。"
            ],
        },
        "chapter": {
            "content_available_via": "run_tool.get_project_information(mode=read, source=current_chapter).result.content",
            "source_spans_available_via": "run_tool.get_project_information(mode=read, source=current_chapter).result.content.source_spans",
            "content_chars": chapter_content.chars().count(),
        },
        "context_manifest": context_manifest.cloned().unwrap_or_else(|| json!({})),
        "available_actions": {
            "think": {
                "arguments": {
                    "focus": "本步要解决的判断焦点",
                    "known_facts": "字符串数组，已经从输入或观察中确认的事实",
                    "information_gaps": "字符串数组，完成当前任务仍缺少的信息",
                    "next_step": "下一步准备执行的动作意图"
                },
                "purpose": "形成可展示、可审计的计划或反思观察；不读取外部信息，不调用工具，不输出最终结果"
            },
            "run_tool": {
                "arguments": {
                    "tool": "tools_prompt 中的一个工具名",
                    "arguments": "符合该工具参数结构的对象"
                },
                "purpose": "执行信息检索或文章补丁工具，结果会作为 observation 返回给下一步"
            },
            "final": {
                "arguments": {
                    "issues": "问题对象数组"
                },
                "purpose": "结束分析并输出可保存的问题列表"
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
        "task": {
            "name": "content_optimization_agent_loop",
            "requirement": "在明确分析模式下自主选择下一步：规划、通过工具获取必要信息、提出文章补丁观察，或输出最终 issues。普通聊天不会进入此流程。",
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
            }
        },
        "output_contract": {
            "root_type": "JSON 对象",
            "required_fields": {
                "issues": "问题对象数组",
            },
            "issue_schema": {
                "id": "非空字符串",
                "tool": "后续处理工具名；正文修改类问题使用 edit_article",
                "arguments": {
                    "evidence_span_id": "必填；只能从读取 current_chapter 后返回的 source_spans 中选择一个 id，不要输出 evidence 原文",
                    "goal": "期望通过局部文章补丁解决的目标"
                },
                "description": "非空字符串",
                "severity": "info|minor|major|critical",
            },
            "forbidden_issue_argument_fields": ["evidence"],
        },
    });
    build_json_task_user_message(
        "执行内容优化 Agent Loop",
        "每次只返回 decision_contract 要求的一个严格 JSON 决策对象，不要输出代码块、解释文字或额外字段；public_update 字符串可以使用简洁 Markdown",
        &payload,
    )
}

pub(super) fn build_content_optimization_patch_user_message(
    base_content_hash: &str,
    issues: &[OptimizeIssue],
    permissions: &OptimizeAgentPermissions,
    agent_instruction: Option<&str>,
) -> Result<String, AppError> {
    let issue_values = issues
        .iter()
        .map(|issue| {
            json!({
                "id": issue.id,
                "tool": issue.tool,
                "description": issue.description,
                "severity": issue.severity,
                "arguments": issue.arguments,
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "agent": {
            "permissions": permissions.as_value(),
            "capabilities": optimization_agent_capabilities(permissions),
            "user_instruction": agent_instruction,
            "write_policy": {
                "auto_apply": false,
                "allowed_operations": ["replace", "delete", "insert_before", "insert_after"],
                "old_text_must_match_once": true,
                "replace_new_text_must_differ_from_old_text": true,
                "full_chapter_rewrite_forbidden": true,
                "max_patch_count": OPTIMIZATION_PATCH_MAX_COUNT,
                "max_old_text_chars": OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
                "max_new_text_chars": OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
            },
        },
        "context_refs": {
            "chapter_content": "稳定上下文包 optimization_patch_chapter_content",
            "project_context": "稳定上下文包 optimization_patch_project_context",
        },
        "chapter": {
            "base_content_hash": base_content_hash,
        },
        "selected_issues": issue_values,
        "task": {
            "name": "content_optimization_patch",
            "requirement": "基于选中的问题生成局部文章补丁。只提出小范围 replace/delete/insert_before/insert_after，不写回数据库，不返回整章正文。",
        },
        "output_contract": {
            "root_type": "JSON 对象",
            "required_fields": {
                "summary": "字符串",
                "patches": "补丁对象数组",
            },
            "empty_patch_policy": "如果没有安全、唯一、局部的替换位置，返回空 patches，并在 summary 说明原因",
            "patch_schema": {
                "patch_id": "非空稳定字符串",
                "issue_id": "选中的 issue id 或 null",
                "operation": "replace|delete|insert_before|insert_after",
                "old_text": "必须从当前章节正文逐字复制、且只出现一次的原文片段；insert_* 中作为定位锚点",
                "new_text": "replace/insert_* 的新文本；delete 时必须为空字符串；replace 时必须与 old_text 有实际差异；不得是整章正文",
                "reason": "为什么这样修改",
                "risk": "low|medium|high 或 null",
            },
        },
    });
    build_json_task_user_message(
        "生成局部修复补丁",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}

pub(super) fn build_content_optimization_chat_user_message(
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
                "arguments": {
                    "reply": "直接给用户看的自然语言回复"
                },
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
            "final_arguments": {
                "reply": "非空字符串"
            }
        },
    });
    build_json_task_user_message(
        "继续内容优化对话 Agent Loop",
        "每次只返回 decision_contract 要求的一个严格 JSON 决策对象，不要输出代码块、解释文字或额外字段；public_update 和 final.reply 字符串可以使用简洁 Markdown",
        &payload,
    )
}

pub(super) fn optimization_chat_history(turns: &[optimization_turn::Model]) -> Value {
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

pub(super) fn is_content_optimization_analysis_prompt(content: &str) -> bool {
    json_user_message_payload(content)
        .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
        .and_then(|payload| {
            payload
                .get("task")
                .and_then(|task| task.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .as_deref()
        == Some("content_optimization_agent_loop")
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

pub(super) fn select_optimization_issues(
    session: &optimization_session::Model,
    issue_ids: Option<Vec<String>>,
) -> Result<Vec<OptimizeIssue>, AppError> {
    let issues_value = session
        .issues
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("优化会话缺少 issues".to_string()))?;
    let all_issues = parse_optimization_issues(&issues_value)?;
    if all_issues.is_empty() {
        return Err(AppError::InvalidArgument(
            "没有可用于生成补丁的问题".to_string(),
        ));
    }
    if matches!(issue_ids.as_ref(), Some(ids) if ids.is_empty()) {
        return Err(AppError::InvalidArgument(
            "至少需要选择一个 issue".to_string(),
        ));
    }
    let targets = match issue_ids {
        Some(ids) => all_issues
            .into_iter()
            .filter(|issue| ids.contains(&issue.id))
            .collect::<Vec<_>>(),
        None => all_issues,
    };
    if targets.is_empty() {
        return Err(AppError::InvalidArgument(
            "选中的 issue 不存在，无法生成补丁".to_string(),
        ));
    }
    Ok(targets)
}

pub(super) fn parse_optimization_issues(raw: &Value) -> Result<Vec<OptimizeIssue>, AppError> {
    let arr = raw
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("issues 必须是数组".to_string()))?;

    let mut out = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 必须是对象")))?;
        reject_unknown_issue_fields(object, idx)?;
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 缺少 id 字段")))?;
        let tool = item
            .get("tool")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 缺少 tool 字段")))?;
        let arguments = item
            .get("arguments")
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 缺少 arguments 字段")))?;
        if !arguments.is_object() {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx}.arguments 必须是对象"
            )));
        }
        let description = item
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("issue {idx} 缺少 description 字段"))
            })?;
        let severity = item
            .get("severity")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| matches!(*s, "info" | "minor" | "major" | "critical"))
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "issue {idx}.severity 必须是 info、minor、major 或 critical"
                ))
            })?;
        out.push(OptimizeIssue {
            id,
            tool,
            arguments,
            description,
            severity,
        });
    }
    Ok(out)
}

fn reject_unknown_issue_fields(
    object: &serde_json::Map<String, Value>,
    idx: usize,
) -> Result<(), AppError> {
    let allowed = ["id", "tool", "arguments", "description", "severity"];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

pub(super) fn materialize_optimization_issue_evidence(
    mut issues: Vec<OptimizeIssue>,
    analysis_context: &Value,
) -> Result<Vec<OptimizeIssue>, AppError> {
    let source_spans = analysis_context
        .get("current_chapter")
        .and_then(|value| value.get("source_spans"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("内容优化上下文缺少 current_chapter.source_spans".into())
        })?;

    for (idx, issue) in issues.iter_mut().enumerate() {
        let arguments = issue.arguments.as_object_mut().ok_or_else(|| {
            AppError::InvalidArgument(format!("issue {idx} arguments 必须是对象"))
        })?;
        if arguments.contains_key("evidence") {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx} arguments.evidence 已废弃；模型必须输出 evidence_span_id"
            )));
        }
        reject_unknown_issue_argument_fields(arguments, idx)?;
        let evidence_span_id = arguments
            .get("evidence_span_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("issue {idx} arguments 缺少 evidence_span_id"))
            })?;
        let evidence = resolve_optimization_source_span_text(source_spans, &evidence_span_id)
            .map_err(|err| AppError::InvalidArgument(format!("issue {idx} {err}")))?;
        arguments.remove("evidence_span_id");
        arguments.insert("evidence".to_string(), Value::String(evidence));
    }

    Ok(issues)
}

fn reject_unknown_issue_argument_fields(
    arguments: &serde_json::Map<String, Value>,
    idx: usize,
) -> Result<(), AppError> {
    let allowed = ["evidence_span_id", "goal"];
    for key in arguments.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx} arguments 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn resolve_optimization_source_span_text(
    source_spans: &[Value],
    source_span_id: &str,
) -> Result<String, String> {
    source_spans
        .iter()
        .find(|span| span.get("id").and_then(Value::as_str) == Some(source_span_id))
        .and_then(|span| span.get("text").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            format!("evidence_span_id 不在 current_chapter.source_spans 中: {source_span_id}")
        })
}

fn preview_chars(value: &str, max_chars: usize) -> String {
    let mut preview = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

pub(super) fn parse_content_optimization_issues(raw: &str) -> Result<Value, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization 模型返回空内容，无法解析为 JSON 对象".to_string(),
        ));
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|e| {
        AppError::InvalidArgument(format!(
            "content_optimization 必须返回严格 JSON 对象: {e}; 输出预览：{}",
            preview_chars(trimmed, 240)
        ))
    })?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("content_optimization 输出必须是 JSON 对象".to_string())
    })?;
    reject_unknown_optimization_root_fields(object)?;
    let issues = object.get("issues").ok_or_else(|| {
        AppError::InvalidArgument("content_optimization 输出缺少 issues 字段".into())
    })?;
    if !issues.is_array() {
        return Err(AppError::InvalidArgument(
            "content_optimization.issues 必须是数组".to_string(),
        ));
    }
    Ok(issues.clone())
}

fn reject_unknown_optimization_root_fields(
    object: &serde_json::Map<String, Value>,
) -> Result<(), AppError> {
    for key in object.keys() {
        if key != "issues" {
            return Err(AppError::InvalidArgument(format!(
                "content_optimization 输出包含未知字段 {key}；只允许 issues"
            )));
        }
    }
    Ok(())
}

pub(super) fn optimization_dimensions(mode: &str) -> Result<&'static str, AppError> {
    match mode {
        "full" => {
            Ok("逻辑连贯性、角色一致性、伏笔呼应、时间线一致性、风格一致性、场景描写、语法与表达")
        }
        "quick" => Ok("阻断阅读或明显影响理解的问题：逻辑断裂、角色矛盾、时间线错误、严重语病"),
        "style_only" => Ok("风格一致性、表达节奏、语气、重复用词、句式单调"),
        "plot_only" => Ok("逻辑连贯性、因果关系、角色动机、伏笔呼应、时间线一致性"),
        other => Err(AppError::InvalidArgument(format!(
            "未知优化分析模式: {other}"
        ))),
    }
}
