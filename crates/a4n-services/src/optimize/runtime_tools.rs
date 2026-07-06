use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::patch::{optimization_content_hash, parse_content_optimization_patch_response};

pub(super) const GET_PROJECT_INFORMATION_TOOL: &str = "get_project_information";
pub(super) const EDIT_ARTICLE_TOOL: &str = "edit_article";

const INFORMATION_SOURCES: &[&str] = &[
    "current_chapter",
    "current_chapter_outline",
    "project_detail",
    "blueprint",
    "part_outlines",
    "chapter_outlines",
    "character_profiles",
    "foreshadowing_index",
    "previous_chapters",
    "next_chapters",
];

pub(super) fn optimization_runtime_tool_specs() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": GET_PROJECT_INFORMATION_TOOL,
                "description": "读取或搜索当前优化会话可访问的项目、章节、章纲、蓝图、角色、伏笔和前后文章节信息。",
                "parameters": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["mode"],
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["read", "search"],
                            "description": "read 按 source 读取完整上下文段；search 按 query 在指定 sources 或全部上下文中搜索。"
                        },
                        "source": {
                            "type": "string",
                            "enum": INFORMATION_SOURCES,
                            "description": "read 模式必填；search 模式可选，不填时搜索全部上下文段。"
                        },
                        "query": {
                            "type": "string",
                            "description": "search 模式必填的关键词。"
                        },
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 20,
                            "description": "search 模式返回的最大匹配数；不填时返回 8 条。"
                        }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": EDIT_ARTICLE_TOOL,
                "description": "提交当前章节的文章补丁，返回待用户审阅的补丁提案；真正写回正文只发生在用户接受并同步补丁之后。",
                "parameters": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["summary", "patches"],
                    "properties": {
                        "summary": {
                            "type": "string",
                            "description": "补丁提案摘要。"
                        },
                        "patches": {
                            "type": "array",
                            "description": "局部补丁列表。",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["patch_id", "operation", "old_text", "new_text", "reason"],
                                "properties": {
                                    "patch_id": { "type": "string" },
                                    "issue_id": { "type": ["string", "null"] },
                                    "operation": {
                                        "type": "string",
                                        "enum": ["replace", "delete", "insert_before", "insert_after"],
                                        "description": "replace=修改；delete=删除 old_text；insert_before/insert_after=在唯一 old_text 锚点前/后新增 new_text。"
                                    },
                                    "old_text": {
                                        "type": "string",
                                        "description": "必须从当前章节正文逐字复制且唯一匹配；插入操作中它是定位锚点。"
                                    },
                                    "new_text": {
                                        "type": "string",
                                        "description": "replace/insert_* 的新文本；delete 时必须为空字符串；replace 时必须与 old_text 有实际差异。"
                                    },
                                    "reason": { "type": "string" },
                                    "risk": { "type": ["string", "null"] }
                                }
                            }
                        }
                    }
                }
            }
        }),
    ]
}

pub(super) fn execute_optimization_runtime_tool(
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

pub(super) fn runtime_tool_reads_current_chapter(tool_name: &str, arguments: &Value) -> bool {
    if tool_name != GET_PROJECT_INFORMATION_TOOL {
        return false;
    }
    arguments.get("mode").and_then(Value::as_str) == Some("read")
        && arguments.get("source").and_then(Value::as_str) == Some("current_chapter")
}

fn execute_get_project_information(arguments: Value, context: &Value) -> Result<Value, AppError> {
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

fn execute_edit_article(
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

fn required_string<'a>(input: &'a Value, field: &str) -> Result<&'a str, AppError> {
    input
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("缺少必填字符串参数: {field}")))
}

fn required_information_source<'a>(input: &'a Value, field: &str) -> Result<&'a str, AppError> {
    let source = required_string(input, field)?;
    if INFORMATION_SOURCES.contains(&source) {
        Ok(source)
    } else {
        Err(AppError::InvalidArgument(format!("未知信息源: {source}")))
    }
}

fn optional_information_source<'a>(
    input: &'a Value,
    field: &str,
) -> Result<Option<&'a str>, AppError> {
    match input.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let source = value.trim();
            if INFORMATION_SOURCES.contains(&source) {
                Ok(Some(source))
            } else {
                Err(AppError::InvalidArgument(format!("未知信息源: {source}")))
            }
        }
        Some(_) => Err(AppError::InvalidArgument(format!("{field} 必须是字符串"))),
    }
}

fn optional_limit(input: &Value) -> Result<usize, AppError> {
    match input.get("limit") {
        None | Some(Value::Null) => Ok(8),
        Some(Value::Number(value)) => {
            let limit = value
                .as_u64()
                .ok_or_else(|| AppError::InvalidArgument("limit 必须是正整数".into()))?;
            if (1..=20).contains(&limit) {
                Ok(limit as usize)
            } else {
                Err(AppError::InvalidArgument("limit 必须在 1..=20 之间".into()))
            }
        }
        Some(_) => Err(AppError::InvalidArgument("limit 必须是整数".into())),
    }
}

fn search_context(context: &Value, source: Option<&str>, query: &str, limit: usize) -> Vec<Value> {
    let query_lower = query.to_lowercase();
    let mut out = Vec::new();
    match source {
        Some(source) => {
            if let Some(value) = context.get(source) {
                collect_matches(source, value, "$", &query_lower, limit, &mut out);
            }
        }
        None => {
            for source in INFORMATION_SOURCES {
                if let Some(value) = context.get(*source) {
                    collect_matches(source, value, "$", &query_lower, limit, &mut out);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    out
}

fn collect_matches(
    source: &str,
    value: &Value,
    path: &str,
    query_lower: &str,
    limit: usize,
    out: &mut Vec<Value>,
) {
    if out.len() >= limit {
        return;
    }
    match value {
        Value::String(text) if text.to_lowercase().contains(query_lower) => {
            out.push(json!({
                "source": source,
                "path": path,
                "snippet": search_snippet(text, query_lower),
            }));
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_matches(
                    source,
                    item,
                    &format!("{path}[{index}]"),
                    query_lower,
                    limit,
                    out,
                );
                if out.len() >= limit {
                    break;
                }
            }
        }
        Value::Object(object) => {
            for (key, child) in object {
                collect_matches(
                    source,
                    child,
                    &format!("{path}.{key}"),
                    query_lower,
                    limit,
                    out,
                );
                if out.len() >= limit {
                    break;
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn search_snippet(text: &str, query_lower: &str) -> String {
    let lower = text.to_lowercase();
    let byte_index = lower.find(query_lower).unwrap_or(0);
    let char_index = lower[..byte_index].chars().count();
    let start = char_index.saturating_sub(60);
    let snippet = text.chars().skip(start).take(140).collect::<String>();
    snippet.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn information_tool_reads_context_section() {
        let context = json!({
            "current_chapter": {
                "content": "第一段正文。第二段出现动机断裂。"
            }
        });
        let result = execute_get_project_information(
            json!({
                "mode": "read",
                "source": "current_chapter"
            }),
            &context,
        )
        .unwrap();

        assert_eq!(result["tool"], GET_PROJECT_INFORMATION_TOOL);
        assert!(
            result["content"]["content"]
                .as_str()
                .unwrap()
                .contains("第二段")
        );
    }

    #[test]
    fn information_tool_searches_across_sources() {
        let context = json!({
            "current_chapter": {
                "content": "陈恪进入旧城。"
            },
            "previous_chapters": [{
                "summary": "旧城线索第一次出现。"
            }]
        });
        let result = execute_get_project_information(
            json!({
                "mode": "search",
                "query": "旧城",
                "limit": 2
            }),
            &context,
        )
        .unwrap();

        assert_eq!(result["matches"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn edit_tool_returns_validated_proposal() {
        let context = json!({
            "current_chapter": {
                "content": "主角收到旧信，决定进入旧城。"
            }
        });
        let result = execute_edit_article(
            json!({
                "summary": "增强动机",
                "patches": [{
                    "patch_id": "patch-001",
                    "issue_id": null,
                    "operation": "replace",
                    "old_text": "主角收到旧信，决定进入旧城",
                    "new_text": "主角收到旧信，又想起信中提到的旧城印记，决定进入旧城",
                    "reason": "补足行动动机",
                    "risk": null
                }]
            }),
            &context,
            1,
            2,
            3,
        )
        .unwrap();

        assert_eq!(result["tool"], EDIT_ARTICLE_TOOL);
        assert_eq!(result["mode"], "pending_confirmation");
        assert_eq!(result["patch_count"], 1);
    }
}
