use a4n_error::AppError;
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

pub(in crate::optimize) fn execute_get_project_information(
    arguments: Value,
    context: &Value,
) -> Result<Value, AppError> {
    let object = arguments
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument("读取参数必须是对象".into()))?;
    for key in object.keys() {
        if ![
            "mode",
            "source",
            "query",
            "limit",
            "offset",
            "path",
            "expected_hash",
        ]
        .contains(&key.as_str())
        {
            return Err(AppError::InvalidArgument(format!("未知读取参数: {key}")));
        }
    }
    let mode = required_string(&arguments, "mode")?;
    match mode {
        "read" => {
            let source = required_information_source(&arguments, "source")?;
            if arguments.get("query").is_some() {
                return Err(AppError::InvalidArgument("read 不接受 query".into()));
            }
            let result = context.get(source).ok_or_else(|| {
                AppError::InvalidArgument(format!("当前会话没有可读取的信息源: {source}"))
            })?;
            super::read::read_source(&arguments, source, result)
        }
        "search" => {
            if ["offset", "path", "expected_hash"]
                .iter()
                .any(|key| arguments.get(key).is_some())
            {
                return Err(AppError::InvalidArgument(
                    "search 不接受 offset/path/expected_hash".into(),
                ));
            }
            let query = required_string(&arguments, "query")?;
            let source = optional_information_source(&arguments, "source")?;
            let limit = optional_limit(&arguments)?;
            let mut matches = search_context(context, source, query, limit);
            for item in &mut matches {
                let source = item["source"]
                    .as_str()
                    .expect("search returns source")
                    .to_string();
                item["resource"] = json!({"kind": "optimization_source", "id": source, "hash": super::read::source_hash(&context[&source])?});
            }
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
