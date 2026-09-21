fn resolve_event_cursor(
    headers: &HeaderMap,
    query_cursor: Option<u64>,
) -> Result<Option<u64>, ApiError> {
    let Some(value) = headers.get(HeaderName::from_static("last-event-id")) else {
        return Ok(query_cursor);
    };
    let raw = value
        .to_str()
        .map_err(|_| ApiError::bad_request("Last-Event-ID 不是有效 ASCII 字符串"))?;
    let header_cursor = parse_sequence(raw, "Last-Event-ID")?;
    if query_cursor.is_some_and(|cursor| cursor != header_cursor) {
        return Err(ApiError::bad_request(
            "Last-Event-ID 必须与 after_sequence 一致",
        ));
    }
    Ok(Some(header_cursor))
}

fn parse_optional_sequence(raw: Option<&str>) -> Result<Option<u64>, ApiError> {
    raw.map(|value| parse_sequence(value, "after_sequence"))
        .transpose()
}

fn parse_sequence(raw: &str, field: &str) -> Result<u64, ApiError> {
    if raw.trim() != raw || raw.is_empty() {
        return Err(ApiError::bad_request(format!("{field} 必须是非负整数")));
    }
    raw.parse::<u64>()
        .map_err(|_| ApiError::bad_request(format!("{field} 必须是非负整数")))
}

fn parse_limit(raw: Option<&str>, default: u32, max: u32) -> Result<u32, ApiError> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    let limit = raw
        .parse::<u32>()
        .map_err(|_| ApiError::bad_request(format!("limit 必须是 1 到 {max} 的整数")))?;
    if !(1..=max).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "limit 必须是 1 到 {max} 的整数"
        )));
    }
    Ok(limit)
}

fn optional_query_text(value: Option<String>, field: &str) -> Result<Option<String>, ApiError> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(ApiError::bad_request(format!("{field} 不能为空")));
            }
            if trimmed.chars().count() > 200 {
                return Err(ApiError::bad_request(format!("{field} 过长")));
            }
            Ok(trimmed.to_string())
        })
        .transpose()
}

fn required_path_id<'a>(value: &'a str, field: &str) -> Result<&'a str, ApiError> {
    if value.is_empty() || value.trim() != value || value.chars().count() > 200 {
        return Err(ApiError::bad_request(format!("path {field} 无效")));
    }
    Ok(value)
}

fn require_matching_id(path: &str, body: &str, field: &str) -> Result<(), ApiError> {
    required_path_id(path, field)?;
    if path != body {
        return Err(ApiError::bad_request(format!(
            "body.{field} 必须与 path {field} 一致"
        )));
    }
    Ok(())
}

fn validate_event_batch(
    thread_id: &str,
    after_sequence: u64,
    events: &[AgentEvent],
) -> Result<(), ApiError> {
    let mut cursor = after_sequence;
    for event in events {
        if event.thread_id != thread_id {
            return Err(ApiError::conflict(
                "Agent event 回放返回了其他 Thread 的事件",
            ));
        }
        if event.sequence <= cursor {
            return Err(ApiError::conflict(
                "Agent event sequence 必须严格递增且大于游标",
            ));
        }
        cursor = event.sequence;
    }
    Ok(())
}
