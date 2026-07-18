use crate::AppState;
use crate::actions::access;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_orchestrator::manga::{
    MangaGeneratePanelsInput, MangaPipelineStartStage, ensure_manga_source_can_write,
    manga_pipeline_business_key, manga_pipeline_fingerprint,
};
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use std::collections::HashMap;

pub async fn run_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    task_id: &str,
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id).await?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_number(project_id, chapter_number, true)
        .await?;
    ensure_manga_source_can_write(&source, "启动漫画流水线")?;
    let reference_context = state
        .orchestrator
        .manga_reference_context(project_id)
        .await?;
    let workflow_fingerprint = manga_pipeline_fingerprint(&source.content, &reference_context);
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: manga_pipeline_business_key(project_id, source.chapter_id),
            workflow_kind: "manga_pipeline".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(source.chapter_id),
            chapter_id: Some(source.chapter_id),
            mode: start_from_stage.map(|stage| stage.key().to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: start_from_stage.map(|stage| stage.key().to_string()),
            active_task_id: Some(task_id.to_string()),
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let variables = HashMap::new();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let result = state
        .generate_panels(MangaGeneratePanelsInput {
            chapter_id: source.chapter_id,
            project_id,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
            reference_context,
            task_id: Some(task_id),
            workflow_run_id: Some(workflow_run.id),
            resume_from_checkpoint,
            start_from_stage,
        })
        .await;
    match &result {
        Ok(_) => {
            state
                .orchestrator
                .mark_workflow_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await?;
        }
        Err(err) => {
            let status = if matches!(err, AppError::Cancelled) {
                WORKFLOW_STATUS_CANCELLED
            } else {
                WORKFLOW_STATUS_FAILED
            };
            state
                .orchestrator
                .mark_workflow_status(workflow_run.id, status, true, Some(&err.to_string()))
                .await
                .map_err(|status_err| {
                    AppError::InvalidArgument(format!(
                        "{err}; 同步漫画工作流状态失败: {status_err}"
                    ))
                })?;
        }
    }
    result
}

pub async fn preflight_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), AppError> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id).await?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_number(project_id, chapter_number, true)
        .await?;
    ensure_manga_source_can_write(&source, "启动漫画流水线")
}
