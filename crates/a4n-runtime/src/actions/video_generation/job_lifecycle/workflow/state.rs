use super::super::super::media::prompt_id_from_video;
use super::super::context::{
    normalized_provider_job_id, video_provider_workflow_business_key,
    video_provider_workflow_fingerprint,
};
use crate::AppState;
use a4n_core::models::ai::generated_video;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_orchestrator::generation::AttachGenerationRunsToWorkflowInput;
use a4n_orchestrator::workflow::{
    UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED,
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
};
use serde_json::{Value, json};

pub(in crate::actions::video_generation::job_lifecycle) async fn interrupt_video_provider_polling(
    state: &AppState,
    video: &generated_video::Model,
    message: &str,
) -> Result<generated_video::Model, String> {
    let updated = state
        .orchestrator
        .mark_generated_video_error_status_and_prompt(
            video.id,
            prompt_id_from_video(video),
            "interrupted",
            message,
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    mark_video_provider_workflow(
        state,
        &updated,
        WORKFLOW_STATUS_INTERRUPTED,
        true,
        Some(message),
    )
    .await?;
    Ok(updated)
}

pub(in crate::actions::video_generation::job_lifecycle) async fn canvas_parent_workflow_is_cancelled(
    state: &AppState,
    video: &generated_video::Model,
) -> Result<bool, String> {
    let Some(workflow_run_id) = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("canvas"))
        .and_then(|canvas| canvas.get("workflow_run_id"))
        .and_then(Value::as_i64)
    else {
        return Ok(false);
    };
    let Some(run) = state
        .orchestrator
        .workflow_run_model_optional_by_id(workflow_run_id)
        .await
        .map_err(crate::actions::app_error_message)?
    else {
        return Ok(false);
    };
    Ok(run.status == WORKFLOW_STATUS_CANCELLED)
}

pub(in crate::actions::video_generation) async fn mark_video_provider_workflow(
    state: &AppState,
    video: &generated_video::Model,
    status: &str,
    can_resume: bool,
    error_message: Option<&str>,
) -> Result<(), String> {
    let Some(job_id) = normalized_provider_job_id(video.provider_job_id.as_deref()) else {
        return Ok(());
    };
    let fingerprint = video_provider_workflow_fingerprint(video, job_id);
    let fingerprint_sha = serde_json::to_string(&fingerprint)
        .map(|value| source_content_sha256(&value))
        .map_err(|err| err.to_string())?;
    let run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: video_provider_workflow_business_key(video.id),
            workflow_kind: "video_provider_job".to_string(),
            project_id: Some(video.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("generated_video".to_string()),
            scope_id: Some(video.id),
            chapter_id: video.chapter_id,
            mode: video.workflow_mode.clone(),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: status.to_string(),
            current_step_key: Some("provider_poll".to_string()),
            active_task_id: None,
            can_resume,
            resume_block_reason: (!can_resume && status != WORKFLOW_STATUS_COMPLETED).then(|| {
                error_message
                    .unwrap_or("该 provider 任务已进入终态，不能继续轮询")
                    .to_string()
            }),
            error_message: error_message.map(str::to_string),
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    state
        .orchestrator
        .upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: run.id,
            step_key: "provider_job".to_string(),
            step_order: 1,
            status: status.to_string(),
            input_fingerprint_sha256: Some(fingerprint_sha.clone()),
            input_fingerprint_json: Some(fingerprint.clone()),
            checkpoint_json: Some(json!({
                "provider_job_id": job_id,
                "generated_video_id": video.id,
                "status": video.status,
            })),
            artifacts_json: Some(provider_job_artifacts_json(video, job_id)),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: is_terminal_provider_workflow_checkpoint_status(status),
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    state
        .orchestrator
        .attach_generation_runs_to_workflow_for_target(AttachGenerationRunsToWorkflowInput {
            target_table: "generated_video".to_string(),
            target_id: video.id,
            workflow_run_id: run.id,
            call_key: Some("provider_job".to_string()),
            input_fingerprint_sha256: Some(fingerprint_sha),
            input_fingerprint_json: Some(fingerprint),
            artifact_refs_json: Some(provider_job_artifacts_json(video, job_id)),
        })
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(())
}

fn provider_job_artifacts_json(video: &generated_video::Model, job_id: &str) -> serde_json::Value {
    json!({
        "targets": [{
            "target_table": "generated_video",
            "target_id": video.id,
            "target_kind": "video_provider_job",
            "role": "provider_job",
            "provider_job_id": job_id,
        }],
    })
}

pub(in crate::actions::video_generation::job_lifecycle) fn is_terminal_provider_workflow_checkpoint_status(
    status: &str,
) -> bool {
    matches!(status, WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_FAILED)
}
