//! Pure projection from workflow persistence summaries to API DTOs.

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
                attempt: run.attempt,
                response_summary_json: run.response_summary_json,
                error_message: run.error_message,
                created_at: run.created_at,
                completed_at: run.completed_at,
            })
            .collect(),
    }
}
