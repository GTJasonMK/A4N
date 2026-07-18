use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_orchestrator::manga::{
    MangaPipelineRequestPreviewInput, MangaPipelineStartStage, ensure_manga_source_can_write,
    manga_pipeline_business_key, manga_pipeline_fingerprint,
};
use std::collections::HashMap;

pub async fn preview_manga_pipeline_requests(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<MangaPipelineStartStage>,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id).await?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_number(project_id, chapter_number, true)
        .await?;
    ensure_manga_source_can_write(&source, "预览漫画流水线提示词")?;
    let reference_context = state
        .orchestrator
        .manga_reference_context(project_id)
        .await?;
    let workflow_fingerprint = manga_pipeline_fingerprint(&source.content, &reference_context);
    let workflow_run_id = state
        .orchestrator
        .latest_workflow_run_by_business_key(&manga_pipeline_business_key(
            project_id,
            source.chapter_id,
        ))
        .await?
        .filter(|run| run.input_fingerprint_json.as_ref() == Some(&workflow_fingerprint))
        .map(|run| run.id);
    let variables = HashMap::new();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let steps = state
        .preview_generate_panels_requests(MangaPipelineRequestPreviewInput {
            chapter_id: source.chapter_id,
            project_id,
            prompt_renderer: &prompt_renderer,
            variables: &variables,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint,
            start_from_stage,
        })
        .await?;
    chat_status_preview_for_state(
        state,
        "manga_pipeline",
        format!("第 {chapter_number} 章漫画流水线请求预览"),
        steps.into_iter().map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        }),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}
