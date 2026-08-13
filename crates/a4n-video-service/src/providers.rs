use crate::contracts::{
    VideoDeleteOutput, VideoGenerationOutput, VideoJobOutput, VideoProviderPayload,
    VideoStatusOutput,
};
use a4n_core::error::AppError;
use a4n_service_protocol::redact_sensitive_message;
use a4n_video::{VideoGenerationResult, VideoJobDeleteOutcome, VideoJobState, VideoJobStatus};
use a4n_video_providers::video_provider_request_body_preview;
use base64::Engine as _;
use serde_json::{Value, json};

pub(crate) fn video_request_preview(payload: &VideoProviderPayload) -> Result<Value, AppError> {
    let body =
        video_provider_request_body_preview(&payload.backend, &payload.base_url, &payload.request)?;
    Ok(json!({ "request": body }))
}

pub(crate) fn video_generation_output(result: VideoGenerationResult) -> VideoGenerationOutput {
    VideoGenerationOutput {
        bytes_base64: base64::engine::general_purpose::STANDARD.encode(result.bytes),
        format: result.format,
        provider_job_id: result.provider_job_id,
        metadata: result.metadata,
    }
}

pub(crate) fn video_job_output(state: VideoJobState) -> VideoJobOutput {
    match state {
        VideoJobState::Pending => VideoJobOutput::Pending,
        VideoJobState::Running => VideoJobOutput::Running,
        VideoJobState::Completed(result) => VideoJobOutput::Completed {
            result: video_generation_output(result),
        },
        VideoJobState::Failed(message) => VideoJobOutput::Failed {
            message: redact_sensitive_message(message),
        },
    }
}

pub(crate) fn video_status_output(status: VideoJobStatus) -> VideoStatusOutput {
    match status {
        VideoJobStatus::Pending => VideoStatusOutput::Pending,
        VideoJobStatus::Running => VideoStatusOutput::Running,
        VideoJobStatus::Completed => VideoStatusOutput::Completed,
        VideoJobStatus::Failed(message) => VideoStatusOutput::Failed {
            message: redact_sensitive_message(message),
        },
        VideoJobStatus::Cancelled => VideoStatusOutput::Cancelled,
        VideoJobStatus::NotFound => VideoStatusOutput::NotFound,
    }
}

pub(crate) fn video_delete_output(outcome: VideoJobDeleteOutcome) -> VideoDeleteOutput {
    match outcome {
        VideoJobDeleteOutcome::Deleted => VideoDeleteOutput::Deleted,
        VideoJobDeleteOutcome::NotFound => VideoDeleteOutput::NotFound,
    }
}
