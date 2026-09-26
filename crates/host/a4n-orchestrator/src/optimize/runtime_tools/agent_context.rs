use a4n_api_contract::optimize::{
    InspectOptimizationContextInput, OPTIMIZATION_CONTEXT_SOURCES, OptimizationContextSelection,
    ReadOptimizationContextInput, SearchOptimizationContextInput,
    canonical_optimization_context_source,
};
use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::OrchestratorContext;
use super::super::context::build_content_optimization_context;
use super::source_hash;
use a4n_api_contract::optimize::OptimizationAgentPermissions;

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
    let result = match source {
        None => json!({
            "kind": "context_index",
            "sources": OPTIMIZATION_CONTEXT_SOURCES.iter().filter_map(|name| {
                context_value.get(*name).map(|value| json!({
                    "source": name,
                    "kind": value_kind(value),
                    "hash": source_hash(value).ok(),
                }))
            }).collect::<Vec<_>>(),
        }),
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
    let query_lower = query.to_lowercase();
    let limit = input.limit.unwrap_or(8).clamp(1, 20) as usize;
    let mut matches = Vec::new();
    let source = input.source.as_deref().map(canonical_source).transpose()?;
    let sources = match source {
        Some(source) => vec![source],
        None => OPTIMIZATION_CONTEXT_SOURCES.to_vec(),
    };
    for source in sources {
        let Some(value) = context_value.get(source) else {
            continue;
        };
        collect_matches(source, value, "$", &query_lower, limit, &mut matches);
        if matches.len() >= limit {
            break;
        }
    }
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
        self.verify_project_permission(
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let permissions = self
            .optimize_sessions_for_user(user_id, project_id, Some(chapter_id))
            .await?
            .into_iter()
            .find_map(|session| {
                session
                    .agent_permissions
                    .and_then(|value| serde_json::from_value(value).ok())
            })
            .unwrap_or_else(default_permissions);
        let mut value = build_content_optimization_context(
            &self.database,
            project_id,
            chapter_id,
            &permissions,
        )
        .await?;
        let chapter = self
            .optimization_chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        value["current_chapter"]["content"] = Value::String(chapter.content);
        Ok(value)
    }
}

fn default_permissions() -> OptimizationAgentPermissions {
    OptimizationAgentPermissions {
        read_project_detail: true,
        read_blueprint: true,
        read_outlines: true,
        read_character_profiles: true,
        read_foreshadowing: true,
        automatic_writeback: false,
    }
}

fn inspect_source(source: &str, value: &Value, cursor: Option<&str>) -> Result<Value, AppError> {
    if source == "current_chapter" {
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::InvalidArgument("current_chapter 缺少正文".into()))?;
        let all_paragraphs = paragraph_ranges(content);
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
                "preview": preview(&serde_json::to_string(value).unwrap_or_default()),
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
    let indexes = (start..=paragraph_ranges(content).len() as u32).collect::<Vec<_>>();
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
    let ranges = paragraph_ranges(content);
    let spans = value
        .get("source_spans")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            let chapter_number = value
                .get("chapter_number")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or_default();
            source_spans_for_content(0, chapter_number, content)
                .into_iter()
                .map(|span| {
                    json!({
                        "id": span.id,
                        "text": span.text,
                        "paragraph_index": span.paragraph_index,
                        "char_start": span.char_start,
                        "char_end": span.char_end,
                    })
                })
                .collect()
        });
    let mut paragraphs = Vec::new();
    for index in indexes.iter().copied().filter(|index| *index > 0) {
        let Some((start, end)) = ranges.get(index as usize - 1).copied() else {
            return Err(AppError::InvalidArgument(format!(
                "paragraph index 超出范围: {index}"
            )));
        };
        let mut paragraph = json!({"index": index});
        if include.contains(&"content") {
            paragraph["content"] = Value::String(content[start..end].to_string());
        }
        if include.contains(&"source_spans") {
            paragraph["source_spans"] = Value::Array(
                spans
                    .iter()
                    .filter(|span| {
                        span.get("paragraph_index").and_then(Value::as_u64) == Some(index as u64)
                    })
                    .cloned()
                    .collect(),
            );
        }
        paragraphs.push(paragraph);
        let candidate = json!({"paragraphs": paragraphs});
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
    Ok(json!({
        "kind": "selected_paragraphs",
        "chapter_number": value.get("chapter_number"),
        "paragraphs": paragraphs,
        "complete": next.is_none(),
        "next_cursor": next,
    }))
}

fn paragraph_cursor(cursor: Option<&str>) -> Result<u32, AppError> {
    match cursor {
        None => Ok(1),
        Some(cursor) => {
            let index = cursor
                .strip_prefix("paragraph:")
                .ok_or_else(|| AppError::InvalidArgument("paragraph cursor 无效".into()))?;
            let index = parse_index(index)?;
            if index == 0 {
                return Err(AppError::InvalidArgument(
                    "paragraph cursor 必须从 1 开始".into(),
                ));
            }
            Ok(index)
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

fn paragraph_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    let mut cursor = 0;
    for line in content.split_inclusive('\n') {
        let line_start = cursor;
        cursor += line.len();
        let trimmed = line.trim_matches(['\n', '\r', ' ', '\t']);
        if trimmed.is_empty() {
            if let Some(begin) = start.take() {
                let end = trim_range_end(content, begin, line_start);
                if begin < end {
                    ranges.push((begin, end));
                }
            }
        } else if start.is_none() {
            start = Some(line_start);
        }
    }
    if let Some(begin) = start {
        let end = trim_range_end(content, begin, content.len());
        if begin < end {
            ranges.push((begin, end));
        }
    }
    if ranges.is_empty() && !content.is_empty() {
        ranges.push((0, content.len()));
    }
    ranges
}

fn parse_index(value: &str) -> Result<u32, AppError> {
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidArgument(format!("cursor 段落索引无效: {value}")))
}

fn trim_range_end(content: &str, start: usize, end: usize) -> usize {
    let trimmed = content[start..end].trim_end_matches(['\n', '\r', ' ', '\t']);
    start + trimmed.len()
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
                "preview": preview(text),
            }));
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_matches(
                    source,
                    item,
                    &format!("{path}/{index}"),
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
                    &format!("{path}/{}", escape_pointer(key)),
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

#[cfg(test)]
mod tests {
    use super::{inspect_source, paragraph_ranges, read_chapter_paragraphs, read_paths};
    use serde_json::json;

    #[test]
    fn paragraph_ranges_keep_blank_line_separated_prose() {
        let content = "第一段第一行。\n第一段第二行。\n\n第二段。\n\n第三段。";
        let ranges = paragraph_ranges(content);
        let paragraphs = ranges
            .iter()
            .map(|(start, end)| content[*start..*end].to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            paragraphs,
            vec!["第一段第一行。\n第一段第二行。", "第二段。", "第三段。"]
        );
    }

    #[test]
    fn paragraph_ranges_return_one_range_for_single_line_content() {
        let content = "正文。";
        assert_eq!(paragraph_ranges(content), vec![(0, content.len())]);
    }

    #[test]
    fn paragraph_reads_preserve_utf8_and_return_continuation_cursor() {
        let content = (0..500).map(|_| "中文段落。\n\n").collect::<String>();
        let value = json!({"chapter_number": 3, "content": content});
        let result = read_chapter_paragraphs(&value, &(1..=500).collect::<Vec<_>>(), &["content"])
            .expect("paragraph read should be bounded");
        assert_eq!(result["paragraphs"][0]["content"], "中文段落。");
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
        let value = json!({"chapter_number": 1, "content": text});
        let page = read_chapter_paragraphs(&value, &[1], &["content", "source_spans"]).unwrap();
        assert_eq!(page["paragraphs"][0]["content"], value["content"]);
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
