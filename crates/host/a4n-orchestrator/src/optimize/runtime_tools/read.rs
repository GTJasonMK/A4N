use a4n_domain::{source_spans::source_spans_for_content, source_text_hash};
use a4n_error::AppError;
use serde_json::{Value, json};

const MAX_READ_BYTES: usize = 16 * 1024;

pub(super) fn read_source(
    arguments: &Value,
    source: &str,
    value: &Value,
) -> Result<Value, AppError> {
    let hash = source_hash(value)?;
    if let Some(expected) = arguments.get("expected_hash")
        && expected.as_str() != Some(hash.as_str())
    {
        return Err(AppError::InvalidArgument(
            "信息源快照已变化，请重新读取首页和 hash".into(),
        ));
    }
    let path = match arguments.get("path") {
        None => "",
        Some(Value::String(path)) if path.is_empty() || path.starts_with('/') => path,
        _ => {
            return Err(AppError::InvalidArgument(
                "path 必须是 JSON pointer，如 /summary".into(),
            ));
        }
    };
    let selected = value
        .pointer(path)
        .ok_or_else(|| AppError::InvalidArgument(format!("信息源不存在路径: {path}")))?;
    let offset = unsigned(arguments, "offset", 0)?;
    let chapter = source == "current_chapter" && path.is_empty();
    let text = if chapter {
        selected.get("content").and_then(Value::as_str)
    } else {
        selected.as_str()
    };
    let (content, total, end, unit) = if let Some(text) = text {
        let limit = bounded_limit(arguments, 2048, 4096)?;
        let total = text.chars().count();
        check_offset(offset, total)?;
        let end = offset.saturating_add(limit).min(total);
        let page = text
            .chars()
            .skip(offset)
            .take(end - offset)
            .collect::<String>();
        let content = if chapter {
            let chapter_number = selected
                .get("chapter_number")
                .and_then(Value::as_i64)
                .and_then(|number| i32::try_from(number).ok())
                .ok_or_else(|| {
                    AppError::InvalidArgument("current_chapter 缺少 chapter_number".into())
                })?;
            let spans = source_spans_for_content(0, chapter_number, text).into_iter()
                .filter(|span| span.char_start >= offset && span.char_end <= end)
                .map(|span| json!({"id": span.id, "text": span.text, "paragraph_index": span.paragraph_index}))
                .collect::<Vec<_>>();
            json!({"chapter_number": chapter_number, "content": page, "source_spans": spans})
        } else {
            json!(page)
        };
        (content, total, end, "characters")
    } else if let Some(items) = selected.as_array() {
        let limit = bounded_limit(arguments, 20, 100)?;
        check_offset(offset, items.len())?;
        let end = offset.saturating_add(limit).min(items.len());
        (json!(&items[offset..end]), items.len(), end, "items")
    } else if let Some(object) = selected.as_object() {
        let limit = bounded_limit(arguments, 20, 100)?;
        check_offset(offset, object.len())?;
        let end = offset.saturating_add(limit).min(object.len());
        let fields = object.keys().skip(offset).take(end - offset).map(|key| {
            json!({"field": key, "path": format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"))})
        }).collect::<Vec<_>>();
        (
            json!({"fields": fields, "message": "用 path 选择需要的字段；文本和数组支持 offset/limit。"}),
            object.len(),
            end,
            "fields",
        )
    } else {
        if offset != 0 || arguments.get("limit").is_some() {
            return Err(AppError::InvalidArgument("标量字段不支持分页".into()));
        }
        (selected.clone(), 1, 1, "value")
    };
    let result = json!({
        "tool": super::GET_PROJECT_INFORMATION_TOOL, "mode": "read", "source": source,
        "resource": {"kind": "optimization_source", "id": source, "hash": hash},
        "path": path, "unit": unit, "offset": offset, "total": total,
        "next_offset": (end < total).then_some(end), "complete": offset == 0 && end == total,
        "content": content,
    });
    if serde_json::to_vec(&result)?.len() > MAX_READ_BYTES {
        return Err(AppError::InvalidArgument(
            "读取结果超过 16 KiB；减小 limit 或用 path 选择数组项/字段后重读".into(),
        ));
    }
    Ok(result)
}

fn unsigned(input: &Value, key: &str, default: usize) -> Result<usize, AppError> {
    match input.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .ok_or_else(|| AppError::InvalidArgument(format!("{key} 必须是非负整数"))),
    }
}

fn bounded_limit(input: &Value, default: usize, max: usize) -> Result<usize, AppError> {
    let limit = unsigned(input, "limit", default)?;
    if !(1..=max).contains(&limit) {
        return Err(AppError::InvalidArgument(format!(
            "limit 必须在 1..={max} 之间"
        )));
    }
    Ok(limit)
}

fn check_offset(offset: usize, total: usize) -> Result<(), AppError> {
    if offset > total {
        return Err(AppError::InvalidArgument("offset 超出信息源范围".into()));
    }
    Ok(())
}

pub(in crate::optimize) fn source_hash(value: &Value) -> Result<String, AppError> {
    Ok(source_text_hash(&serde_json::to_string(value)?))
}

/// Evidence is restricted to complete spans actually returned by a successful
/// read of this snapshot. A search snippet or partial sentence is not evidence.
pub(in crate::optimize) fn observed_evidence_context(
    context: &Value,
    observations: &[Value],
) -> Result<Value, AppError> {
    let chapter = context
        .get("current_chapter")
        .ok_or_else(|| AppError::InvalidArgument("缺少 current_chapter".into()))?;
    let hash = source_hash(chapter)?;
    let mut spans = Vec::new();
    for observation in observations {
        if observation.get("success").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let Some(result) = observation.get("result") else {
            continue;
        };
        if result.get("source").and_then(Value::as_str) != Some("current_chapter")
            || result.get("mode").and_then(Value::as_str) != Some("read")
            || result.pointer("/resource/hash").and_then(Value::as_str) != Some(hash.as_str())
        {
            continue;
        }
        if let Some(read) = result
            .pointer("/content/source_spans")
            .and_then(Value::as_array)
        {
            spans.extend(read.iter().cloned());
        } else if result.get("path").and_then(Value::as_str) == Some("/source_spans")
            && let Some(read) = result.get("content").and_then(Value::as_array)
        {
            spans.extend(read.iter().cloned());
        }
    }
    Ok(json!({"current_chapter": {"source_spans": spans}}))
}
