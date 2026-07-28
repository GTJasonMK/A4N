pub use a4n_db::generation::{
    AttachGenerationRunsToWorkflowInput, GENERATION_ENTITY_KINDS, GENERATION_ENTITY_ROLES,
    GENERATION_MEDIA_REF_ROLES, GENERATION_PROMPT_KINDS, GENERATION_SPEC_STATUSES,
    GENERATION_SPEC_TARGET_ENTITY_KINDS, GENERATION_SPEC_TARGET_KINDS,
    GENERATION_TEXT_ANCHOR_ROLES, GenerationRunCheckpointContext, GenerationRunTraceTarget,
    LlmCallUsageTraceInput, PromptArtifactInsertInput, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, RecordLlmCallTraceInput,
    RecordStartedGenerationRunInput, SPEC_STATUS_DRAFT, generation_checkpoint_fingerprint_sha256,
    is_valid_generation_entity_kind, is_valid_generation_entity_role,
    is_valid_generation_media_ref_role, is_valid_generation_prompt_kind,
    is_valid_generation_spec_status, is_valid_generation_spec_target_entity_kind,
    is_valid_generation_spec_target_kind, is_valid_generation_text_anchor_role, render_prompt_text,
};

pub mod artifacts;
pub mod checkpoint;
pub mod events;
pub mod stream;
pub mod terminal;

use a4n_core::error::AppError;
use a4n_service_protocol::{GenerationError, GenerationErrorCode, GenerationFinishReason};

pub(crate) fn generation_error(error: a4n_db::generation::GenerationDbError) -> AppError {
    match error {
        a4n_db::generation::GenerationDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::generation::GenerationDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::generation::GenerationDbError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn generation_public_error(
    code: &str,
    stage_code: &str,
    message: impl ToString,
) -> GenerationError {
    GenerationError {
        code: GenerationErrorCode::registered(code).expect("registered generation error code"),
        message: message.to_string().chars().take(4_000).collect(),
        retryable: matches!(
            code,
            "generation_checkpoint_unavailable"
                | "generation_interrupted"
                | "generation_persist_failed"
                | "provider_job_failed"
        ),
        stage_code: stage_code.to_string(),
        trace_id: None,
        source_error: None,
    }
}

pub(crate) fn generation_finish_reason_text(reason: GenerationFinishReason) -> String {
    match reason {
        GenerationFinishReason::Stop => "stop",
        GenerationFinishReason::ToolCallsComplete => "tool_calls",
    }
    .to_string()
}
