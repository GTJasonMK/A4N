use std::collections::HashMap;

use a4n_core::{
    blueprint::{
        BLUEPRINT_REFINEMENT_MODE, BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
        blueprint_refinement_business_key, blueprint_refinement_call_key,
    },
    blueprint_context::blueprint_llm_context,
    error::AppError,
};
use a4n_db::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_prompt::blueprint_generation::build_blueprint_refine_request;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::contracts::PreparedBlueprintRefine;

pub(super) async fn prepare_blueprint_refine(
    context: &OrchestratorContext,
    project_id: i64,
    task_id: &str,
    focus: Option<&str>,
    prompt_renderer: &dyn PromptRenderer,
    variables: &HashMap<String, Value>,
) -> Result<PreparedBlueprintRefine, AppError> {
    let source_rows = a4n_db::outline::load_outline_generation_source_rows(
        context.database.connection(),
        project_id,
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if source_rows.project.lifecycle_gate != a4n_db::generation::PROJECT_GATE_ACTIVE {
        return Err(AppError::InvalidState {
            current: source_rows.project.lifecycle_gate.clone(),
            operation: "优化故事蓝图".to_string(),
        });
    }
    let expected_source_snapshot =
        a4n_db::outline::outline_generation_source_snapshot(&source_rows);
    let blueprint = source_rows
        .blueprint
        .clone()
        .ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;
    let blueprint_context = blueprint_llm_context(
        &blueprint,
        &source_rows.blueprint_characters,
        &source_rows.blueprint_relationships,
    );

    let mut request = build_blueprint_refine_request(
        prompt_renderer,
        project_id,
        focus,
        variables,
        &blueprint_context,
    )
    .await?;
    let call_key = blueprint_refinement_call_key(project_id);
    let request_json = context_aware_chat_request_trace_json(&request)?;
    let input_fingerprint_json = json!({
        "schema_version": 1,
        "workflow": BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
        "call_key": &call_key,
        "request": request_json,
        "source_snapshot": &expected_source_snapshot,
    });
    let workflow_run = context
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: blueprint_refinement_business_key(project_id),
            workflow_kind: BLUEPRINT_REFINEMENT_WORKFLOW_KIND.to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_blueprint".to_string()),
            scope_id: Some(blueprint.id),
            chapter_id: None,
            mode: Some(BLUEPRINT_REFINEMENT_MODE.to_string()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: Some(task_id.to_string()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let workflow_run_id = workflow_run.id;
    crate::workflow::attach_llm_call_checkpoint_trace(
        &mut request,
        workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )?;
    context
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;

    Ok(PreparedBlueprintRefine {
        expected_source_snapshot,
        request,
        input_fingerprint_json,
        call_key,
        workflow_run_id,
    })
}
