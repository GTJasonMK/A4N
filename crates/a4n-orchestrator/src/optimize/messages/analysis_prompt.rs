use a4n_core::error::AppError;
use a4n_prompt::request::{build_json_task_user_message, json_user_message_payload};
use serde_json::{Value, json};

use crate::optimize::OptimizeAgentPermissions;
use crate::optimize::permissions::optimization_agent_capabilities;

pub(in crate::optimize) fn build_content_optimization_user_message(
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
                "arguments": {"issues": "问题对象数组"},
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
            "required_fields": {"issues": "问题对象数组"},
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

pub(in crate::optimize) fn is_content_optimization_analysis_prompt(content: &str) -> bool {
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

pub(in crate::optimize) fn optimization_dimensions(mode: &str) -> Result<&'static str, AppError> {
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
