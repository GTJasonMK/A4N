use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::rules::*;
use super::search::search_context;
use super::specs::{EDIT_ARTICLE_TOOL, GET_PROJECT_INFORMATION_TOOL};
use crate::optimize::patch::{
    optimization_content_hash, parse_content_optimization_patch_response,
};

pub(in crate::optimize) fn execute_optimization_runtime_tool(
    tool_name: &str,
    arguments: Value,
    context: &Value,
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Value, AppError> {
    match tool_name {
        GET_PROJECT_INFORMATION_TOOL => execute_get_project_information(arguments, context),
        EDIT_ARTICLE_TOOL => {
            execute_edit_article(arguments, context, session_id, project_id, chapter_id)
        }
        other => Err(AppError::InvalidArgument(format!(
            "未注册的优化运行时工具: {other}"
        ))),
    }
}

pub(in crate::optimize) fn runtime_tool_reads_current_chapter(
    tool_name: &str,
    arguments: &Value,
) -> bool {
    if tool_name != GET_PROJECT_INFORMATION_TOOL {
        return false;
    }
    arguments.get("mode").and_then(Value::as_str) == Some("read")
        && arguments.get("source").and_then(Value::as_str) == Some("current_chapter")
}

pub(in crate::optimize) fn execute_get_project_information(
    arguments: Value,
    context: &Value,
) -> Result<Value, AppError> {
    let mode = required_string(&arguments, "mode")?;
    match mode {
        "read" => {
            let source = required_information_source(&arguments, "source")?;
            let result = context.get(source).cloned().ok_or_else(|| {
                AppError::InvalidArgument(format!("当前会话没有可读取的信息源: {source}"))
            })?;
            Ok(json!({
                "tool": GET_PROJECT_INFORMATION_TOOL,
                "mode": "read",
                "source": source,
                "content": result,
            }))
        }
        "search" => {
            let query = required_string(&arguments, "query")?;
            let source = optional_information_source(&arguments, "source")?;
            let limit = optional_limit(&arguments)?;
            let matches = search_context(context, source, query, limit);
            Ok(json!({
                "tool": GET_PROJECT_INFORMATION_TOOL,
                "mode": "search",
                "source": source,
                "query": query,
                "matches": matches,
            }))
        }
        other => Err(AppError::InvalidArgument(format!(
            "get_project_information.mode 非法: {other}"
        ))),
    }
}

pub(in crate::optimize) fn execute_edit_article(
    arguments: Value,
    context: &Value,
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Value, AppError> {
    let content = context
        .get("current_chapter")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AppError::InvalidArgument("edit_article 需要 current_chapter.content".into())
        })?;
    let base_content_hash = optimization_content_hash(content);
    let raw = serde_json::to_string(&json!({
        "summary": required_string(&arguments, "summary")?,
        "patches": arguments
            .get("patches")
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("edit_article 缺少 patches".into()))?,
    }))?;
    let proposal = parse_content_optimization_patch_response(
        session_id,
        project_id,
        chapter_id,
        &base_content_hash,
        content,
        &raw,
    )?;
    Ok(json!({
        "tool": EDIT_ARTICLE_TOOL,
        "mode": "pending_confirmation",
        "requires_user_confirmation": true,
        "chapter_id": chapter_id,
        "base_content_hash": proposal.base_content_hash,
        "patch_count": proposal.patches.len(),
        "summary": proposal.summary,
        "patches": proposal.patches,
    }))
}
