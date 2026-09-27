use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::json;

use super::OptimizeIssue;

pub(in crate::optimize) fn build_content_optimization_patch_user_message(
    base_content_hash: &str,
    issues: &[OptimizeIssue],
) -> Result<String, AppError> {
    let payload = json!({
        "task": {
            "name": "content_optimization_patch",
            "requirement": "针对 selected_issues 生成局部文章补丁，遵守 system 的输出合同。",
        },
        "context_refs": {
            "chapter_content": "稳定上下文包 optimization_patch_chapter_content",
            "project_context": "稳定上下文包 optimization_patch_project_context",
        },
        "chapter": {"base_content_hash": base_content_hash},
        "selected_issues": issues,
    });
    build_json_task_user_message(
        "生成局部修复补丁",
        "只返回 system 要求的严格 JSON 对象",
        &payload,
    )
}
