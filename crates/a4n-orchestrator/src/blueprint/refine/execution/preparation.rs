use std::collections::HashMap;

use a4n_core::{blueprint_context::blueprint_llm_context, error::AppError};
use a4n_db::workflow::{self, UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_prompt::blueprint_generation::{
    blueprint_refine_manifest_user_message, build_blueprint_refine_request,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::workflow::workflow_write_app_error;

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

    let prompt_content = prompt_renderer
        .render_prompt("blueprint_optimization", variables, Some(project_id))
        .await?;
    let user_message =
        blueprint_refine_manifest_user_message(prompt_renderer, Some(project_id), focus).await?;
    let mut request = build_blueprint_refine_request(
        project_id,
        prompt_content,
        user_message,
        &blueprint_context,
    )?;
    let request_json = context_aware_chat_request_trace_json(&request)?;
    let input_fingerprint_json = json!({
        "prompt_names": ["blueprint_optimization", "blueprint_optimization_user"],
        "request": request_json,
        "input_variables": variables,
        "focus": focus,
        "source_refs": {
            "blueprint_id": blueprint.id,
            "current_version": &blueprint.current_version,
            "blueprint_context": &blueprint_context,
            "source_snapshot": &expected_source_snapshot,
        },
    });
    let call_key = format!("blueprint_refine:project:{project_id}");
    let workflow_run = workflow::upsert_workflow_run(
        context.database.connection(),
        UpsertWorkflowRunInput {
            business_key: format!("blueprint_refine:{project_id}"),
            workflow_kind: "blueprint_refine".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_blueprint".to_string()),
            scope_id: Some(blueprint.id),
            chapter_id: None,
            mode: focus.map(str::to_string),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: Some(task_id.to_string()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .map_err(workflow_write_app_error)?;
    let workflow_run_id = workflow_run.id;
    crate::workflow::attach_llm_call_checkpoint_trace(
        &mut request,
        workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )?;
    workflow::mark_workflow_current_step(context.database.connection(), workflow_run_id, &call_key)
        .await
        .map_err(workflow_write_app_error)?;

    Ok(PreparedBlueprintRefine {
        expected_source_snapshot,
        request,
        input_fingerprint_json,
        call_key,
        workflow_run_id,
    })
}
