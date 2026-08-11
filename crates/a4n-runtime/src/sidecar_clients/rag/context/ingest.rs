use super::scope::rag_operation_timestamp;

pub(crate) fn rag_ingest_operation_trace_id(
    project_id: i64,
    task_id: Option<&str>,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err(format!(
            "RAG ingest project_id 必须是正整数，实际为 {project_id}"
        ));
    }
    if let Some(task_id) = task_id {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Err("RAG ingest task_id 不能为空".to_string());
        }
        if task_id.len() > 160
            || !task_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
        {
            return Err(
                "RAG ingest task_id 只能包含 ASCII 字母、数字、-、_、.、:，且不能超过 160 字节"
                    .to_string(),
            );
        }
        return Ok(format!("task:{task_id}"));
    }

    Ok(format!(
        "rag_ingest:{project_id}:{}",
        rag_operation_timestamp()?
    ))
}

pub(crate) fn rag_client_ingest_operation_trace_id(project_id: i64) -> Result<String, String> {
    if project_id <= 0 {
        return Err(format!(
            "RAG client ingest project_id 必须是正整数，实际为 {project_id}"
        ));
    }
    Ok(format!(
        "rag_client_ingest:{project_id}:{}",
        rag_operation_timestamp()?
    ))
}

pub(crate) fn validate_rag_client_ingest_operation_trace_id(
    project_id: i64,
    trace_id: &str,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err(format!(
            "RAG client ingest project_id 必须是正整数，实际为 {project_id}"
        ));
    }
    let trace_id = trace_id.trim();
    let prefix = format!("rag_client_ingest:{project_id}:");
    let suffix = trace_id.strip_prefix(&prefix).ok_or_else(|| {
        format!("RAG client ingest operation_trace_id 与 project_id={project_id} 不匹配")
    })?;
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("RAG client ingest operation_trace_id 的 run 部分必须是数字".to_string());
    }
    Ok(trace_id.to_string())
}
