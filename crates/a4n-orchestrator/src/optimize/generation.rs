use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_service_protocol::{GenerationError, GenerationTaskStatus, GenerationUsage};

use crate::generation::{
    events::{GenerationSessionError, GenerationTaskSession},
    generation_public_error,
    terminal::GenerationTerminalError,
};

use super::AgentLoopExecutionError;

pub(in crate::optimize) use crate::generation::generation_progress as optimize_progress;

pub(in crate::optimize) struct GenerationStreamCompletion {
    pub(in crate::optimize) raw_utf8_bytes: u64,
    pub(in crate::optimize) raw_sha256: String,
    pub(in crate::optimize) final_usage: Option<GenerationUsage>,
}

pub(in crate::optimize) fn generation_error_from_session(
    error: GenerationSessionError,
) -> GenerationError {
    generation_public_error(error.code(), "checkpoint", error)
}

pub(in crate::optimize) fn generation_error_from_terminal(
    error: GenerationTerminalError,
) -> GenerationError {
    generation_public_error(error.code(), "persist", error)
}

pub(in crate::optimize) fn agent_loop_error_to_generation_error(
    error: AgentLoopExecutionError,
) -> GenerationError {
    match error {
        AgentLoopExecutionError::Generation(error) => error,
        AgentLoopExecutionError::App(error) => app_error_to_generation_error(error, "agent_loop"),
    }
}

pub(in crate::optimize) fn app_error_to_generation_error(
    error: AppError,
    stage_code: &str,
) -> GenerationError {
    let (code, stage) = match error {
        AppError::Cancelled => ("generation_interrupted", "provider_stream"),
        AppError::LlmError(_) | AppError::Timeout(_) => ("provider_job_failed", "provider_stream"),
        AppError::Database(_) | AppError::Serialization(_) => {
            ("generation_persist_failed", "persist")
        }
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            ("generation_task_forbidden", stage_code)
        }
        AppError::InvalidArgument(_) | AppError::InvalidState { .. } => {
            ("generation_request_invalid", stage_code)
        }
        _ => ("generation_persist_failed", stage_code),
    };
    generation_public_error(code, stage, error)
}

pub(in crate::optimize) fn generation_stream_completion(
    session: &Arc<GenerationTaskSession>,
) -> Result<GenerationStreamCompletion, GenerationError> {
    let stream = session
        .snapshot()
        .streams
        .into_iter()
        .find(|stream| stream.stream_id == "raw_json")
        .ok_or_else(|| {
            generation_public_error(
                "generation_checkpoint_unavailable",
                "checkpoint",
                "Optimize generation 缺少 raw_json generation stream",
            )
        })?;
    let raw_utf8_bytes = u64::try_from(stream.text.len()).map_err(|_| {
        generation_public_error(
            "generation_event_invalid",
            "checkpoint",
            "Optimize raw_json 长度超出 generation 协议范围",
        )
    })?;
    Ok(GenerationStreamCompletion {
        raw_utf8_bytes,
        raw_sha256: stream.sha256,
        final_usage: session.snapshot().usage,
    })
}

pub(in crate::optimize) fn is_generation_cancelled(
    session_status: GenerationTaskStatus,
    error: &GenerationError,
) -> bool {
    session_status == GenerationTaskStatus::Cancelling
        || error.code.as_str() == "generation_interrupted"
}
