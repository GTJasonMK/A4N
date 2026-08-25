pub use a4n_db::generation::{LlmCallUsageTraceInput, RecordLlmCallTraceInput};

pub(crate) mod artifacts;
pub(crate) mod checkpoint;
pub(crate) mod events;
pub(crate) mod stream;
pub(crate) mod terminal;

pub use artifacts::GeneratedMediaArtifactFile;
pub use events::{GenerationReplay, GenerationSessionError, GenerationSubscription};
pub use terminal::{GenerationTerminalError, GenerationTerminalWriteOutcome};

use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_domain::progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationError, GenerationErrorCode, GenerationFinishReason,
    GenerationResultRef, GenerationTaskStatus, ProgressEvent,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
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

pub(crate) fn generation_result_ref<T: Serialize + ?Sized>(
    kind: &str,
    id: i64,
    value: &T,
) -> Result<GenerationResultRef, AppError> {
    Ok(GenerationResultRef {
        kind: kind.to_string(),
        id: id.to_string(),
        revision: format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)),
    })
}

pub(crate) fn generation_batch_result_ref(
    kind: &str,
    id: i64,
    mut item_ids: Vec<i64>,
) -> Result<GenerationResultRef, AppError> {
    item_ids.sort_unstable();
    generation_result_ref(kind, id, &item_ids)
}

pub(crate) fn best_effort_batch_summary(
    total: u32,
    succeeded: u32,
    failed: u32,
    cancelled: u32,
    skipped: u32,
    failed_item_keys: &[String],
) -> BatchSummary {
    let mut failed_item_keys = failed_item_keys.to_vec();
    failed_item_keys.sort_unstable();
    failed_item_keys.dedup();
    let failed_item_keys_truncated = failed_item_keys.len() > 256;
    failed_item_keys.truncate(256);
    BatchSummary {
        policy: BatchCommitPolicy::BestEffort,
        total,
        succeeded,
        failed,
        cancelled,
        skipped,
        failed_item_keys,
        failed_item_keys_truncated,
    }
}

pub(crate) fn generation_failure_is_cancelled_or_interrupted(
    session_status: GenerationTaskStatus,
    error: &GenerationError,
) -> bool {
    session_status == GenerationTaskStatus::Cancelling
        || error.code.as_str() == "generation_interrupted"
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

pub(crate) fn generation_finish_reason_from_text(reason: &str) -> Option<GenerationFinishReason> {
    match reason {
        "stop" => Some(GenerationFinishReason::Stop),
        "tool_calls" => Some(GenerationFinishReason::ToolCallsComplete),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_reason_text_round_trips() {
        for reason in [
            GenerationFinishReason::Stop,
            GenerationFinishReason::ToolCallsComplete,
        ] {
            assert_eq!(
                generation_finish_reason_from_text(&generation_finish_reason_text(reason)),
                Some(reason)
            );
        }
        assert_eq!(generation_finish_reason_from_text("length"), None);
    }

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

    #[test]
    fn generation_failure_distinguishes_cancel_or_interrupt_from_failure() {
        let failed = generation_public_error("provider_job_failed", "provider_stream", "failed");
        let interrupted =
            generation_public_error("generation_interrupted", "provider_stream", "interrupted");

        assert!(generation_failure_is_cancelled_or_interrupted(
            GenerationTaskStatus::Cancelling,
            &failed,
        ));
        assert!(generation_failure_is_cancelled_or_interrupted(
            GenerationTaskStatus::Running,
            &interrupted,
        ));
        assert!(!generation_failure_is_cancelled_or_interrupted(
            GenerationTaskStatus::Running,
            &failed,
        ));
    }
}
