use a4n_error::AppError;
use a4n_llm::trace::context_aware_chat_request_trace_json;

use super::contracts::VideoPromptGenerationTrace;

pub(super) fn prepare_generation_trace(
    trace: VideoPromptGenerationTrace,
) -> Result<a4n_db::video_prompt::PreparedVideoPromptGenerationTrace, AppError> {
    let request_json = context_aware_chat_request_trace_json(&trace.request)?;
    let rendered_messages_json = serde_json::to_value(&trace.request.messages).map_err(|err| {
        AppError::InvalidArgument(format!("视频提示词 LLM 消息序列化失败: {err}"))
    })?;
    let usage_json = trace
        .usage
        .map(|usage| {
            serde_json::to_value(usage).map_err(|err| {
                AppError::InvalidArgument(format!("视频提示词 LLM usage 序列化失败: {err}"))
            })
        })
        .transpose()?;
    Ok(a4n_db::video_prompt::PreparedVideoPromptGenerationTrace {
        prompt_name: trace.prompt_name,
        request_json,
        rendered_messages_json,
        response_text: trace.response_text,
        workflow_run_id: trace.workflow_run_id,
        usage_json,
    })
}
