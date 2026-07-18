use super::super::super::media::prompt_id_from_video;
use a4n_core::models::ai::generated_video;
use serde_json::{Value, json};

pub(crate) fn video_provider_workflow_business_key(video_id: i64) -> String {
    format!("video_provider_job:{video_id}")
}

pub(crate) fn video_provider_workflow_fingerprint(
    video: &generated_video::Model,
    job_id: &str,
) -> Value {
    json!({
        "generated_video_id": video.id,
        "provider_job_id": job_id,
        "prompt_id": prompt_id_from_video(video),
        "config_id": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("config_id"))
            .and_then(serde_json::Value::as_i64),
        "workflow_mode": video.workflow_mode,
        "source_prompt_type": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("source_prompt_type")),
        "video_execution_identity": video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_execution_identity")),
    })
}
