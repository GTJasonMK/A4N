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

use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_domain::progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_service_protocol::{
    GenerationError, GenerationErrorCode, GenerationFinishReason, ProgressEvent,
};
use tokio::sync::mpsc;

use self::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::{GenerationEventPublisher, GenerationTaskSession, SessionGenerationEventPublisher},
};

pub(crate) struct GenerationProgressUpdate {
    pub(crate) current: u32,
    pub(crate) total: u32,
    pub(crate) message: String,
}

pub(crate) async fn publish_step_progress(
    mut receiver: mpsc::UnboundedReceiver<GenerationProgressUpdate>,
    publisher: SessionGenerationEventPublisher,
    checkpoint_writer: GenerationCheckpointWriter,
    session: Arc<GenerationTaskSession>,
    operation_key: &'static str,
    checkpoint_stage_code: &'static str,
    active_phase: ProgressPhaseKind,
    minimum_reported_total: u32,
) -> Result<(), GenerationError> {
    while let Some(update) = receiver.recv().await {
        publisher
            .progress(step_progress_event(
                update,
                operation_key,
                active_phase,
                minimum_reported_total,
            ))
            .await
            .map_err(|error| generation_public_error(error.code(), operation_key, error))?;
        checkpoint_writer
            .persist_if_due(
                &session,
                GenerationCheckpointMetadata {
                    step_key: Some(operation_key.to_string()),
                    ..Default::default()
                },
                false,
            )
            .await
            .map_err(|error| generation_public_error(error.code(), checkpoint_stage_code, error))?;
    }
    checkpoint_writer
        .persist_if_due(
            &session,
            GenerationCheckpointMetadata {
                step_key: Some(operation_key.to_string()),
                ..Default::default()
            },
            true,
        )
        .await
        .map_err(|error| generation_public_error(error.code(), checkpoint_stage_code, error))?;
    Ok(())
}

fn step_progress_event(
    update: GenerationProgressUpdate,
    operation_key: &str,
    active_phase: ProgressPhaseKind,
    minimum_reported_total: u32,
) -> ProgressEvent {
    let (phase_kind, severity) = step_progress_state(update.current, update.total, active_phase);
    ProgressEvent {
        stage_code: operation_key.to_string(),
        phase_kind,
        message: update.message,
        current: Some(update.current.into()),
        total: Some(update.total.max(minimum_reported_total).into()),
        unit_label: Some("步骤".to_string()),
        item_key: None,
        item_index: None,
        item_total: None,
        severity,
        estimated: false,
    }
}

pub(crate) fn step_progress_state(
    current: u32,
    total: u32,
    active_phase: ProgressPhaseKind,
) -> (ProgressPhaseKind, ProgressSeverity) {
    let complete = total > 0 && current >= total;
    let phase_kind = if current == 0 {
        ProgressPhaseKind::Prepare
    } else if complete {
        ProgressPhaseKind::Complete
    } else {
        active_phase
    };
    let severity = if complete {
        ProgressSeverity::Success
    } else {
        ProgressSeverity::Info
    };
    (phase_kind, severity)
}

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

pub(crate) fn generation_session_error(error: events::GenerationSessionError) -> GenerationError {
    generation_public_error(error.code(), "checkpoint", error)
}

pub(crate) fn generation_terminal_error(
    error: terminal::GenerationTerminalError,
) -> GenerationError {
    generation_public_error(error.code(), "persist", error)
}

pub(crate) fn generation_progress(
    stage_code: &str,
    phase_kind: ProgressPhaseKind,
    message: &str,
) -> ProgressEvent {
    ProgressEvent {
        stage_code: stage_code.to_string(),
        phase_kind,
        message: message.to_string(),
        current: None,
        total: None,
        unit_label: None,
        item_key: None,
        item_index: None,
        item_total: None,
        severity: ProgressSeverity::Info,
        estimated: false,
    }
}

pub(crate) fn generation_finish_reason_text(reason: GenerationFinishReason) -> String {
    match reason {
        GenerationFinishReason::Stop => "stop",
        GenerationFinishReason::ToolCallsComplete => "tool_calls",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_progress_preserves_active_phase_and_minimum_reported_total() {
        let event = step_progress_event(
            GenerationProgressUpdate {
                current: 0,
                total: 0,
                message: "准备".to_string(),
            },
            "video_generate",
            ProgressPhaseKind::AssetGeneration,
            1,
        );

        assert_eq!(event.phase_kind, ProgressPhaseKind::Prepare);
        assert_eq!(event.severity, ProgressSeverity::Info);
        assert_eq!(event.total, Some(1));

        let event = step_progress_event(
            GenerationProgressUpdate {
                current: 1,
                total: 2,
                message: "生成中".to_string(),
            },
            "video_generate",
            ProgressPhaseKind::AssetGeneration,
            0,
        );
        assert_eq!(event.phase_kind, ProgressPhaseKind::AssetGeneration);
    }
}
