use a4n_core::blueprint::BLUEPRINT_GENERATE_PROGRESS_TOTAL;
use a4n_core::error::AppError;
use a4n_orchestrator::blueprint::BlueprintGenerationInput;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state};
use crate::actions::template_variables::parse_template_variables_app_error as parse_template_variables;
use crate::actions::workflow_status_write_app_error;
use crate::prompt_renderer::RuntimePromptRenderer;

use super::contracts::{
    BlueprintProgressReporter, BlueprintSummary, BlueprintTaskScope, GenerateBlueprintInput,
};
use super::projections::{blueprint_preview_step, blueprint_summary};

pub async fn generate_blueprint_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateBlueprintInput,
) -> Result<BlueprintTaskScope, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    Ok(BlueprintTaskScope {
        project_id: input.project_id,
        total: BLUEPRINT_GENERATE_PROGRESS_TOTAL,
    })
}

pub async fn get_blueprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<BlueprintSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    Ok(state
        .orchestrator
        .novel_blueprint_for_project(project_id)
        .await?
        .map(|blueprint| blueprint_summary(&blueprint)))
}

pub async fn generate_blueprint(
    state: &AppState,
    user_id: i64,
    input: GenerateBlueprintInput,
    cancel_token: Option<&CancellationToken>,
    progress: Option<BlueprintProgressReporter<'_>>,
) -> Result<BlueprintSummary, AppError> {
    generate_blueprint_task_scope(state, user_id, &input).await?;
    report(
        progress,
        0,
        BLUEPRINT_GENERATE_PROGRESS_TOTAL,
        "启动蓝图生成",
    )?;
    let variables = parse_template_variables(&input.variables)?;
    let workflow_fingerprint = json!({
        "schema_version": 1,
        "workflow": "blueprint_generation",
        "project_id": input.project_id,
        "title": &input.title,
        "genre": &input.genre,
        "target_length": input.target_length,
        "variables": &variables,
    });
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("blueprint_generation:project:{}", input.project_id),
            workflow_kind: "blueprint_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(input.project_id),
            chapter_id: None,
            mode: Some("generate".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("blueprint_generation.story_core".to_string()),
            active_task_id: input.task_id.clone(),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let workflow_run_id = workflow_run.id;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let blueprint_result = state
        .orchestrator
        .generate_blueprint(BlueprintGenerationInput {
            project_id: input.project_id,
            novel_title: &input.title,
            genre: &input.genre,
            target_length: input.target_length,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
            cancel_token,
            progress,
            workflow_run_id: Some(workflow_run_id),
        })
        .await;
    match blueprint_result {
        Ok(blueprint) => {
            state
                .orchestrator
                .mark_workflow_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await?;
            Ok(blueprint_summary(&blueprint))
        }
        Err(err) => {
            let status = if matches!(err, AppError::Cancelled) {
                WORKFLOW_STATUS_CANCELLED
            } else {
                WORKFLOW_STATUS_FAILED
            };
            if let Err(status_error) = state
                .orchestrator
                .mark_workflow_status(workflow_run_id, status, true, Some(&err.to_string()))
                .await
            {
                return Err(workflow_status_write_app_error(&err, &status_error));
            }
            Err(err)
        }
    }
}

pub async fn preview_generate_blueprint_request(
    state: &AppState,
    user_id: i64,
    input: GenerateBlueprintInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(&input.variables)?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let stages = state
        .orchestrator
        .preview_blueprint_generation_requests(
            input.project_id,
            &input.title,
            &input.genre,
            input.target_length,
            &prompt_renderer,
            &variables,
        )
        .await?;
    chat_status_preview_for_state(
        state,
        "blueprint_generation",
        "蓝图生成请求预览",
        stages.into_iter().map(blueprint_preview_step),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}

fn report(
    progress: Option<BlueprintProgressReporter<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress {
        progress(current, total.max(1), message)?;
    }
    Ok(())
}
