use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::json;

use super::OptimizeIssue;
use crate::optimize::permissions::optimization_agent_capabilities;
use crate::optimize::{
    OPTIMIZATION_PATCH_MAX_COUNT, OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
    OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS, OptimizeAgentPermissions,
};

pub(in crate::optimize) fn build_content_optimization_patch_user_message(
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
        "chapter": {"base_content_hash": base_content_hash},
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
