use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatChunk, ToolCall, UsageInfo};
use a4n_llm_sidecar_contract::LlmStreamChunk;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};

use super::super::config::llm_sidecar_error;

pub(in crate::dynamic_llm) enum SidecarLlmEvent {
    Chunk(ChatChunk),
    Progress,
    Completed,
}

pub(in crate::dynamic_llm) fn parse_sidecar_llm_event(
    event: CapabilityTaskEvent,
) -> Result<SidecarLlmEvent, AppError> {
    match event.event_kind {
        CapabilityTaskEventKind::Chunk => {
            let payload = event.payload.ok_or_else(|| {
                AppError::LlmError("LLM sidecar chunk 事件缺少 payload".to_string())
            })?;
            parse_stream_chunk(payload).map(SidecarLlmEvent::Chunk)
        }
        CapabilityTaskEventKind::Completed => Ok(SidecarLlmEvent::Completed),
        CapabilityTaskEventKind::Failed => {
            let error = event.error.ok_or_else(|| {
                AppError::LlmError("LLM sidecar failed 事件缺少 error".to_string())
            })?;
            Err(llm_sidecar_error(error))
        }
        CapabilityTaskEventKind::Progress => Ok(SidecarLlmEvent::Progress),
    }
}

fn parse_stream_chunk(payload: serde_json::Value) -> Result<ChatChunk, AppError> {
    let chunk: LlmStreamChunk = serde_json::from_value(payload)?;
    Ok(ChatChunk {
        content: chunk.content,
        reasoning_content: chunk.reasoning_content,
        tool_calls: chunk.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|call| ToolCall {
                    id: call.id,
                    name: call.name,
                    arguments: call.arguments,
                    stream_index: call.stream_index,
                    provider_metadata: call.provider_metadata,
                })
                .collect()
        }),
        finish_reason: chunk.finish_reason,
        index: chunk.index,
        usage: chunk.usage.map(|usage| UsageInfo {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            prompt_cache_hit_tokens: usage.prompt_cache_hit_tokens,
            prompt_cache_miss_tokens: usage.prompt_cache_miss_tokens,
            prompt_cache_write_tokens: usage.prompt_cache_write_tokens,
        }),
    })
}
