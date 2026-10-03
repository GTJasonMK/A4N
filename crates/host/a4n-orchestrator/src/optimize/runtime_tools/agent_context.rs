use a4n_api_contract::optimize::{
    InspectOptimizationContextInput, OPTIMIZATION_CONTEXT_SOURCES, OptimizationContextSelection,
    ReadOptimizationContextInput, SearchOptimizationContextInput,
    canonical_optimization_context_source,
};
use a4n_domain::source_spans::source_paragraph_ranges;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::OrchestratorContext;
use super::super::context::build_content_optimization_context;
use super::super::permissions::{default_agent_permissions, optimization_permissions_from_session};
use super::read::current_chapter_source_data;
use super::search::search_context;
use super::source_hash;

const CONTEXT_RESULT_BYTES: usize = 12 * 1024;
const PREVIEW_CHARS: usize = 120;

pub(in crate::optimize) async fn inspect_project_context(
    context: &OrchestratorContext,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: InspectOptimizationContextInput,
) -> Result<Value, AppError> {
    let context_value = context
        .agent_context_value(user_id, project_id, chapter_id)
        .await?;
    if let Some(expected_hash) = input.expected_hash.as_deref()
        && expected_hash
            != source_hash_for_requested_source(&context_value, input.source.as_deref())?
    {
        return Err(AppError::InvalidArgument(
            "信息源快照已变化，请重新检查当前上下文目录".into(),
        ));
    }
    let source = input.source.as_deref().map(canonical_source).transpose()?;
    if input.cursor.is_some() && source != Some("current_chapter") {
        return Err(AppError::InvalidArgument(
            "cursor 只适用于当前章节的段落目录".into(),
        ));
    }
    let result = match source {
        None => {
            let sources = OPTIMIZATION_CONTEXT_SOURCES
                .iter()
                .filter_map(|name| context_value.get(*name).map(|value| (name, value)))
                .map(|(name, value)| {
                    Ok(json!({
                        "source": name,
                        "kind": value_kind(value),
                        "hash": source_hash(value)?,
                    }))
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            json!({"kind": "context_index", "sources": sources})
        }
        Some(source) => {
            let value = context_value.get(source).ok_or_else(|| {
                AppError::InvalidArgument(format!("当前会话没有可读取的信息源: {source}"))
            })?;
            inspect_source(source, value, input.cursor.as_deref())?
        }
    };
    let result = bounded_value(result)?;
    Ok(json!({
        "tool": "inspect_project_context",
        "source": source,
        "resource": {
            "kind": "optimization_source",
            "id": source.unwrap_or("context"),
            "hash": source_hash_for_requested_source(&context_value, source)?,
        },
        "content": result,
    }))
}

pub(in crate::optimize) async fn read_project_context(
    context: &OrchestratorContext,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: ReadOptimizationContextInput,
) -> Result<Value, AppError> {
    let context_value = context
        .agent_context_value(user_id, project_id, chapter_id)
        .await?;
    let source = canonical_source(&input.source)?;
    let value = context_value.get(source).ok_or_else(|| {
        AppError::InvalidArgument(format!("当前会话没有可读取的信息源: {source}"))
    })?;
    let hash = source_hash(value)?;
    if input
        .expected_hash
        .as_deref()
        .is_some_and(|expected| expected != hash)
    {
        return Err(AppError::InvalidArgument(
            "信息源快照已变化，请重新检查上下文后读取".into(),
        ));
    }
    let include = normalized_include(&input.include)?;
    if input.cursor.is_some()
        && !matches!(
            (&input.selection, source),
            (
                OptimizationContextSelection::All | OptimizationContextSelection::Paragraphs { .. },
                "current_chapter"
            )
        )
    {
        return Err(AppError::InvalidArgument(
            "cursor 只适用于当前章节的段落读取".into(),
        ));
    }
    let content = match (&input.selection, source) {
        (OptimizationContextSelection::All, "current_chapter") => {
            read_all_chapter(value, &include, input.cursor.as_deref())?
        }
        (OptimizationContextSelection::Paragraphs { indexes }, "current_chapter") => {
            if indexes.contains(&0) {
                return Err(AppError::InvalidArgument(
                    "paragraph index 必须从 1 开始".into(),
                ));
            }
            let start = paragraph_cursor(input.cursor.as_deref())?;
            let mut indexes = indexes
                .iter()
                .copied()
                .filter(|index| *index >= start)
                .collect::<Vec<_>>();
            indexes.sort_unstable();
            indexes.dedup();
            read_chapter_paragraphs(value, &indexes, &include)?
        }
        (OptimizationContextSelection::Paths { paths }, "current_chapter") => {
            if paths
                .iter()
                .any(|path| path.is_empty() || path == "/content")
            {
                return Err(AppError::InvalidArgument(
                    "current_chapter 正文请通过 paragraphs 选择读取，以返回来源短标签".into(),
                ));
            }
            read_paths(value, paths, &include)?
        }
        (OptimizationContextSelection::Paths { paths }, _) => read_paths(value, paths, &include)?,
        (OptimizationContextSelection::Items { indexes }, _) => read_items(value, indexes)?,
        (OptimizationContextSelection::All, _) => bounded_value(value.clone())?,
        _ => {
            return Err(AppError::InvalidArgument(
                "paragraphs 选择只适用于 current_chapter".into(),
            ));
        }
    };
    Ok(json!({
        "tool": "read_project_context",
        "mode": "read",
        "source": source,
        "resource": {"kind": "optimization_source", "id": source, "hash": hash},
        "selection": input.selection,
        "content": content,
    }))
}

pub(in crate::optimize) async fn search_project_context(
    context: &OrchestratorContext,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: SearchOptimizationContextInput,
) -> Result<Value, AppError> {
    let context_value = context
        .agent_context_value(user_id, project_id, chapter_id)
        .await?;
    let query = input.query.trim();
    if query.is_empty() {
        return Err(AppError::InvalidArgument("query 不能为空".into()));
    }
    let limit = input.limit.unwrap_or(8).clamp(1, 20) as usize;
    let source = input.source.as_deref().map(canonical_source).transpose()?;
    let mut matches = search_context(&context_value, source, query, limit);
    for item in &mut matches {
        let source =
            item.get("source")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::InvalidState {
                    current: "上下文搜索结果缺少 source".into(),
                    operation: "构造 Agent 上下文搜索结果".into(),
                })?;
        item["resource"] = json!({
            "kind": "optimization_source",
            "id": source,
            "hash": source_hash(&context_value[source])?,
        });
    }
    Ok(json!({
        "tool": "search_project_context",
        "mode": "search",
        "source": source,
        "query": query,
        "matches": matches,
    }))
}

impl OrchestratorContext {
    async fn agent_context_value(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Value, AppError> {
        let permissions = match self
            .agent_optimization_session_for_user(user_id, project_id, chapter_id)
            .await?
        {
            Some(session) => optimization_permissions_from_session(&session)?,
            None => default_agent_permissions(),
        };
        build_content_optimization_context(
            self.database.connection(),
            project_id,
            chapter_id,
            &permissions,
        )
        .await
    }
}

fn inspect_source(source: &str, value: &Value, cursor: Option<&str>) -> Result<Value, AppError> {
    if source == "current_chapter" {
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::InvalidArgument("current_chapter 缺少正文".into()))?;
        let all_paragraphs = source_paragraph_ranges(content);
        let start = paragraph_cursor(cursor)?;
        let mut paragraphs = Vec::new();
        for (offset, (start_byte, end_byte)) in all_paragraphs
            .iter()
            .copied()
            .enumerate()
            .skip(start.saturating_sub(1) as usize)
        {
            let paragraph = &content[start_byte..end_byte];
            let candidate = json!({
                "index": offset + 1,
                "chars": paragraph.chars().count(),
                "preview": preview(paragraph),
            });
            paragraphs.push(candidate);
            if serde_json::to_vec(&json!({"paragraphs": paragraphs}))?.len() > CONTEXT_RESULT_BYTES
            {
                paragraphs.pop();
                break;
            }
        }
        let last = paragraphs
            .last()
            .and_then(|value| value.get("index"))
            .and_then(Value::as_u64)
            .unwrap_or(u64::from(start.saturating_sub(1)));
        let next_cursor = all_paragraphs
            .iter()
            .enumerate()
            .find(|(index, _)| (*index as u64) + 1 > last)
            .map(|(index, _)| format!("paragraph:{}", index + 1));
        return Ok(json!({
            "kind": "chapter_index",
            "metadata": {
                "chapter_number": value.get("chapter_number"),
                "title": value.get("title"),
                "content_chars": content.chars().count(),
                "paragraph_count": all_paragraphs.len(),
            },
            "paragraphs": paragraphs,
            "complete": next_cursor.is_none(),
            "next_cursor": next_cursor,
        }));
    }
    if let Some(object) = value.as_object() {
        return Ok(json!({
            "kind": "object_index",
            "fields": object.iter().map(|(key, value)| json!({
                "path": format!("/{}", escape_pointer(key)),
                "type": value_kind(value),
                "chars": value.as_str().map(|text| text.chars().count()),
            })).collect::<Vec<_>>(),
        }));
    }
    if let Some(items) = value.as_array() {
        return Ok(json!({
            "kind": "array_index",
            "total": items.len(),
            "items": items.iter().enumerate().map(|(index, value)| json!({
                "index": index,
                "type": value_kind(value),
                "preview": preview(&value.to_string()),
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(json!({"kind": value_kind(value), "value": value}))
}

fn read_all_chapter(
    value: &Value,
    include: &[&str],
    cursor: Option<&str>,
) -> Result<Value, AppError> {
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("current_chapter 缺少正文".into()))?;
    let start = paragraph_cursor(cursor)?;
    let indexes = (start..=source_paragraph_ranges(content).len() as u32).collect::<Vec<_>>();
    read_chapter_paragraphs(value, &indexes, include)
}

fn read_chapter_paragraphs(
    value: &Value,
    indexes: &[u32],
    include: &[&str],
) -> Result<Value, AppError> {
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("current_chapter 缺少正文".into()))?;
    let ranges = source_paragraph_ranges(content);
    let (spans, source_markers) = current_chapter_source_data(value)?;
    let mut paragraphs = Vec::new();
    for index in indexes.iter().copied() {
        let Some((start, end)) = ranges.get(index as usize - 1).copied() else {
            return Err(AppError::InvalidArgument(format!(
                "paragraph index 超出范围: {index}"
            )));
        };
        let paragraph_char_start = content[..start].chars().count();
        let paragraph_spans = spans
            .iter()
            .filter(|span| span.paragraph_index == index as usize)
            .collect::<Vec<_>>();
        let mut paragraph = json!({"index": index});
        if include.contains(&"content") {
            let marker_ranges = paragraph_spans
                .iter()
                .map(|span| {
                    (
                        span.id.clone(),
                        span.char_start - paragraph_char_start,
                        span.char_end - paragraph_char_start,
                        span.text.clone(),
                    )
                })
                .collect::<Vec<_>>();
            paragraph["content"] =
                Value::String(source_markers.mark_ranges(&content[start..end], marker_ranges)?);
        }
        if include.contains(&"source_spans") {
            paragraph["source_span_ids"] = json!(
                paragraph_spans
                    .iter()
                    .map(|span| span.id.as_str())
                    .collect::<Vec<_>>()
            );
        }
        paragraphs.push(paragraph);
        let candidate = source_markers.encode_input(&json!({
            "kind": "selected_paragraphs",
            "chapter_number": value.get("chapter_number"),
            "paragraphs": &paragraphs,
            "complete": false,
            "next_cursor": "paragraph:4294967295",
        }))?;
        if serde_json::to_vec(&candidate)?.len() > CONTEXT_RESULT_BYTES {
            // Always make progress for an oversized single paragraph. The
            // Agent execution owner stores the complete result and exposes its
            // existing Unicode result reader; do not create a second pager or
            // return an empty page with the same cursor forever.
            if paragraphs.len() > 1 {
                paragraphs.pop();
            }
            break;
        }
    }
    let last = paragraphs
        .last()
        .and_then(|value| value.get("index"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next = indexes
        .iter()
        .copied()
        .find(|index| u64::from(*index) > last)
        .map(|index| format!("paragraph:{index}"));
    let result = json!({
        "kind": "selected_paragraphs",
        "chapter_number": value.get("chapter_number"),
        "paragraphs": paragraphs,
        "complete": next.is_none(),
        "next_cursor": next,
    });
    source_markers.encode_input(&result)
}

fn paragraph_cursor(cursor: Option<&str>) -> Result<u32, AppError> {
    match cursor {
        None => Ok(1),
        Some(cursor) => {
            let index = cursor
                .strip_prefix("paragraph:")
                .ok_or_else(|| AppError::InvalidArgument("paragraph cursor 无效".into()))?;
            parse_index(index)
        }
    }
}

fn read_paths(value: &Value, paths: &[String], include: &[&str]) -> Result<Value, AppError> {
    let mut selected = Vec::with_capacity(paths.len());
    for path in paths {
        if !(path.is_empty() || path.starts_with('/')) {
            return Err(AppError::InvalidArgument(format!(
                "path 必须是 JSON Pointer: {path}"
            )));
        }
        let item = value
            .pointer(path)
            .ok_or_else(|| AppError::InvalidArgument(format!("信息源不存在路径: {path}")))?;
        selected.push(json!({
            "path": path,
            "value": if include.contains(&"content") { item.clone() } else { Value::Null },
            "type": value_kind(item),
        }));
    }
    bounded_value(Value::Array(selected))
}

fn read_items(value: &Value, indexes: &[u32]) -> Result<Value, AppError> {
    let items = value
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("items 选择要求信息源是数组".into()))?;
    let selected = indexes
        .iter()
        .copied()
        .map(|index| {
            items
                .get(index as usize)
                .cloned()
                .map(|value| json!({"index": index, "value": value}))
                .ok_or_else(|| AppError::InvalidArgument(format!("数组项超出范围: {index}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    bounded_value(Value::Array(selected))
}

fn bounded_value(value: Value) -> Result<Value, AppError> {
    if serde_json::to_vec(&value)?.len() > CONTEXT_RESULT_BYTES {
        return Err(AppError::InvalidArgument(
            "选择结果过大，请减少选择范围或使用 cursor 继续读取".into(),
        ));
    }
    Ok(value)
}

fn normalized_include(values: &[String]) -> Result<Vec<&str>, AppError> {
    if values.is_empty() {
        return Ok(vec!["content", "metadata", "source_spans"]);
    }
    let values = values.iter().map(String::as_str).collect::<Vec<_>>();
    if values
        .iter()
        .any(|value| !matches!(*value, "content" | "metadata" | "source_spans"))
    {
        return Err(AppError::InvalidArgument(
            "include 只允许 content、metadata 或 source_spans".into(),
        ));
    }
    Ok(values)
}

fn source_hash_for_requested_source(
    context: &Value,
    source: Option<&str>,
) -> Result<String, AppError> {
    match source.map(canonical_source).transpose()? {
        Some(source) => context
            .get(source)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("当前会话没有可读取的信息源: {source}"))
            })
            .and_then(source_hash),
        None => source_hash(context),
    }
}

fn canonical_source(source: &str) -> Result<&'static str, AppError> {
    canonical_optimization_context_source(source)
        .ok_or_else(|| AppError::InvalidArgument(format!("未知信息源: {}", source.trim())))
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn parse_index(value: &str) -> Result<u32, AppError> {
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("cursor 段落索引无效: {value}")))
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn preview(value: &str) -> String {
    let mut chars = value.chars();
    let mut text = chars.by_ref().take(PREVIEW_CHARS).collect::<String>();
    if chars.next().is_some() {
        text.push('…');
    }
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{CONTEXT_RESULT_BYTES, inspect_source, read_chapter_paragraphs, read_paths};
    use serde_json::{Value, json};

    fn chapter_value(content: &str) -> Value {
        json!({"chapter_number": 3, "content": content})
    }

    #[test]
    fn selected_paragraph_returns_its_own_evidence() {
        let value = chapter_value("\r\n第一段。\r\n第二段。\r\n\r\n第三段。");
        let page = read_chapter_paragraphs(&value, &[2], &["content", "source_spans"]).unwrap();
        assert_eq!(page["paragraphs"][0]["content"], "#1 第二段。");
        assert_eq!(page["paragraphs"][0]["source_span_ids"], json!(["#1"]));
        assert!(page["source_marker_protocol"].is_string());
        assert!(!page.to_string().contains("ch3:s"));

        let unindexed = read_chapter_paragraphs(
            &chapter_value("无引用。"),
            &[1],
            &["content", "source_spans"],
        )
        .unwrap();
        assert_eq!(unindexed["paragraphs"][0]["content"], "#0 无引用。");
    }

    #[test]
    fn paragraph_reads_preserve_utf8_and_return_continuation_cursor() {
        let content = (0..500).map(|_| "中文段落。\n\n").collect::<String>();
        let value = chapter_value(&content);
        let result = read_chapter_paragraphs(&value, &(1..=500).collect::<Vec<_>>(), &["content"])
            .expect("paragraph read should be bounded");
        assert_eq!(result["paragraphs"][0]["content"], "#0 中文段落。");
        assert!(serde_json::to_vec(&result).unwrap().len() <= CONTEXT_RESULT_BYTES);
        assert_eq!(result["complete"], false);
        assert!(
            result["next_cursor"]
                .as_str()
                .is_some_and(|value| value.starts_with("paragraph:"))
        );
    }

    #[test]
    fn oversized_paragraph_makes_progress_and_history_cursor_never_rewinds() {
        let text = format!("{}末尾约束", "长段落。".repeat(4000));
        let value = chapter_value(&text);
        let page = read_chapter_paragraphs(&value, &[1], &["content", "source_spans"]).unwrap();
        let marked_content = page["paragraphs"][0]["content"].as_str().unwrap();
        assert!(marked_content.starts_with("#0 长段落。"));
        assert!(marked_content.ends_with("末尾约束"));
        assert_eq!(page["complete"], true);
        let end = inspect_source("current_chapter", &value, Some("paragraph:2")).unwrap();
        assert_eq!(end["complete"], true);
        assert!(end["next_cursor"].is_null());
        assert!(inspect_source("current_chapter", &value, Some("invalid")).is_err());
    }

    #[test]
    fn paths_use_json_pointer_and_reject_missing_values() {
        let value = json!({"metadata": {"title": "标题"}});
        let selected = read_paths(&value, &["/metadata/title".into()], &["content"])
            .expect("pointer should resolve");
        assert_eq!(selected[0]["value"], "标题");
        assert!(read_paths(&value, &["/metadata/missing".into()], &["content"]).is_err());
    }

    #[test]
    fn chapter_index_paginates_before_tool_result_grows_too_large() {
        let content = (0..1000).map(|_| "中文段落。\n\n").collect::<String>();
        let value = json!({"chapter_number": 3, "content": content});
        let page = inspect_source("current_chapter", &value, None).expect("index should fit");
        assert_eq!(page["metadata"]["paragraph_count"], 1000);
        assert_eq!(page["complete"], false);
        assert!(
            page["next_cursor"]
                .as_str()
                .is_some_and(|cursor| cursor.starts_with("paragraph:"))
        );
    }
}
