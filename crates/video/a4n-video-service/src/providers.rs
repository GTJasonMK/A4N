use a4n_service_protocol::redact_sensitive_message;
use a4n_video_provider_contract::{
    VideoGenerationResult, VideoJobDeleteOutcome, VideoJobState, VideoJobStatus,
};
use a4n_video_sidecar_contract::{
    VideoDeleteOutput, VideoGenerationOutput, VideoJobOutput, VideoStatusOutput,
};
use base64::Engine as _;

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
