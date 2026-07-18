pub fn chapter_stream_task_id(
    project_id: i64,
    chapter_number: i32,
    provided: Option<String>,
) -> Result<String, String> {
    let Some(candidate) = provided.map(|id| id.trim().to_string()) else {
        return Ok(format!(
            "chapter-{project_id}-{chapter_number}-{}",
            chrono::Utc::now().timestamp_millis()
        ));
    };
    if candidate.is_empty() {
        return Err("task_id 不能为空".to_string());
    }
    if candidate.len() > 160 {
        return Err("task_id 长度不能超过 160 个字符".to_string());
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err("task_id 只能包含 ASCII 字母、数字、-、_、:、.".to_string());
    }
    Ok(candidate)
}
