use super::super::*;

pub(super) fn chat_chunk_from_sidecar_event(
    event: ServiceTaskEvent,
) -> Result<ChatChunk, AppError> {
    if let Some(payload) = event.payload {
        return serde_json::from_value(payload).map_err(AppError::Serialization);
    }

    let index = match event.chunk_index {
        Some(index) => u32::try_from(index).map_err(|_| {
            AppError::Internal(anyhow::anyhow!(
                "LLM sidecar chunk_index 超出 u32 范围: {index}"
            ))
        })?,
        None => 0,
    };
    let usage = event
        .usage_delta
        .map(usage_from_sidecar_value)
        .transpose()?;

    Ok(ChatChunk {
        content: event.chunk_delta,
        reasoning_content: None,
        tool_calls: None,
        finish_reason: event.finish_reason,
        index,
        usage,
    })
}

pub(super) fn usage_from_sidecar_value(value: serde_json::Value) -> Result<UsageInfo, AppError> {
    serde_json::from_value(value).map_err(AppError::Serialization)
}

pub(super) fn sidecar_event_error(event: ServiceTaskEvent) -> AppError {
    event
        .error
        .map(llm_sidecar_error)
        .unwrap_or_else(|| AppError::LlmError("LLM sidecar stream 返回失败事件".to_string()))
}

pub(in crate::dynamic_llm) fn sidecar_failed_event(
    task_id: String,
    error: ServiceError,
) -> ServiceTaskEvent {
    ServiceTaskEvent {
        task_id,
        event_kind: ServiceTaskEventKind::Failed,
        sequence: 0,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: Some(error),
        payload: None,
    }
}
