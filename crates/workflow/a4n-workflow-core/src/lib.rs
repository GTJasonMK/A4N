//! Pure projection from workflow persistence summaries to API DTOs.

use a4n_error::AppError;
use a4n_video_contract::VideoSegmentExecutionRecipeV1;

pub fn workflow_run_summary_to_api(
    run: a4n_db::workflow::WorkflowRunSummary,
) -> a4n_api_contract::workflow::WorkflowRunSummary {
    a4n_api_contract::workflow::WorkflowRunSummary {
        id: run.id,
        business_key: run.business_key,
        workflow_kind: run.workflow_kind,
        project_id: run.project_id,
        root_project_id: run.root_project_id,
        source_snapshot_id: run.source_snapshot_id,
        scope_table: run.scope_table,
        scope_id: run.scope_id,
        chapter_id: run.chapter_id,
        mode: run.mode,
        input_fingerprint_json: run.input_fingerprint_json,
        status: run.status,
        current_step_key: run.current_step_key,
        progress: run.progress.map(|progress| {
            a4n_api_contract::workflow::WorkflowRunProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message,
                percent: progress.percent,
                meta: progress.meta,
                updated_at: progress.updated_at,
            }
        }),
        active_task_id: run.active_task_id,
        can_resume: run.can_resume,
        resume_block_reason: run.resume_block_reason,
        error_message: run.error_message,
        created_at: run.created_at,
        updated_at: run.updated_at,
        completed_at: run.completed_at,
    }
}

pub async fn update_video_segment_plan_draft_recipe_checkpoint(
    db: &impl a4n_db::DbExecutor,
    project_id: i64,
    workflow_run_id: i64,
    source_shot_id: &str,
    segment_index_in_shot: i32,
    style: &str,
    recipe: VideoSegmentExecutionRecipeV1,
) -> Result<(), AppError> {
    let source_shot_id = source_shot_id.trim().to_string();
    if source_shot_id.is_empty() || source_shot_id.contains(':') {
        return Err(AppError::InvalidArgument(
            "片段规划草稿 source_shot_id 无效".to_string(),
        ));
    }
    if segment_index_in_shot <= 0 {
        return Err(AppError::InvalidArgument(
            "segment_index_in_shot 必须为正整数".to_string(),
        ));
    }
    let step_key = format!("video_segment_plan.shot.{source_shot_id}");
    a4n_db::workflow::mutate_completed_workflow_checkpoint_artifacts_unless_step_exists(
        db,
        project_id,
        workflow_run_id,
        &step_key,
        "video_segment_plan.final",
        move |parsed_output_json, artifacts_json| {
            a4n_video_contract::update_video_segment_plan_draft_recipe_artifacts(
                parsed_output_json,
                artifacts_json,
                &source_shot_id,
                segment_index_in_shot,
                style,
                recipe,
            )
            .map_err(workflow_write_error)
        },
    )
    .await
    .map_err(workflow_write_app_error)
}

fn workflow_write_error(error: AppError) -> a4n_db::workflow::WorkflowWriteError {
    match error {
        AppError::InvalidArgument(message) => {
            a4n_db::workflow::WorkflowWriteError::InvalidArgument(message)
        }
        AppError::InvalidState { current, operation } => {
            a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation }
        }
        AppError::NotFound(message) => a4n_db::workflow::WorkflowWriteError::NotFound(message),
        other => a4n_db::workflow::WorkflowWriteError::InvalidData(other.to_string()),
    }
}

fn workflow_write_app_error(error: a4n_db::workflow::WorkflowWriteError) -> AppError {
    match error {
        a4n_db::workflow::WorkflowWriteError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::workflow::WorkflowWriteError::NotFound(message) => AppError::NotFound(message),
        a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        other => AppError::InvalidState {
            current: other.to_string(),
            operation: "更新视频片段规划草稿".to_string(),
        },
    }
}

pub fn workflow_run_detail_to_api(
    detail: a4n_db::workflow::WorkflowRunDetailSummary,
) -> a4n_api_contract::workflow::WorkflowRunDetailSummary {
    a4n_api_contract::workflow::WorkflowRunDetailSummary {
        run: workflow_run_summary_to_api(detail.run),
        checkpoints: detail
            .checkpoints
            .into_iter()
            .map(
                |checkpoint| a4n_api_contract::workflow::WorkflowCheckpointSummary {
                    id: checkpoint.id,
                    workflow_run_id: checkpoint.workflow_run_id,
                    step_key: checkpoint.step_key,
                    step_order: checkpoint.step_order,
                    status: checkpoint.status,
                    input_fingerprint_sha256: checkpoint.input_fingerprint_sha256,
                    input_fingerprint_json: checkpoint.input_fingerprint_json,
                    checkpoint_json: checkpoint.checkpoint_json,
                    artifacts_json: checkpoint.artifacts_json,
                    generation_run_ids_json: checkpoint.generation_run_ids_json,
                    error_message: checkpoint.error_message,
                    created_at: checkpoint.created_at,
                    updated_at: checkpoint.updated_at,
                    completed_at: checkpoint.completed_at,
                },
            )
            .collect(),
        generation_runs: detail
            .generation_runs
            .into_iter()
            .map(|run| a4n_api_contract::workflow::GenerationRunSummary {
                id: run.id,
                workflow_run_id: run.workflow_run_id,
                workflow: run.workflow,
                step_key: run.step_key,
                call_key: run.call_key,
                title: run.title,
                model_kind: run.model_kind,
                provider: run.provider,
                model: run.model,
                status: run.status,
                input_fingerprint_sha256: run.input_fingerprint_sha256,
                input_fingerprint_json: run.input_fingerprint_json,
                raw_response_json: run.raw_response_json,
                parsed_output_json: run.parsed_output_json,
                validation_json: run.validation_json,
                artifact_refs_json: run.artifact_refs_json,
                has_raw_response: run.has_raw_response,
                has_parsed_output: run.has_parsed_output,
                has_validation: run.has_validation,
                attempt: run.attempt,
                response_summary_json: run.response_summary_json,
                error_message: run.error_message,
                created_at: run.created_at,
                completed_at: run.completed_at,
            })
            .collect(),
        generation_run_count: detail.generation_run_count,
        checkpoint_details_included: detail.checkpoint_details_included,
    }
}
