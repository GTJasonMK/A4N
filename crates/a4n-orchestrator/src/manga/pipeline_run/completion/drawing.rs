use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::PIPELINE_TOTAL_STEPS;
use super::super::super::checkpoint::checkpoint_insert;
use super::super::super::pipeline_json::validate_page_drawing_prompts;
use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::contracts::{
    MangaPlanningRun, MangaRunContext, MangaRunningPipelineCheckpointInput,
};
use super::super::drawing_prompt::DrawingPromptStepInput;
use super::contracts::MangaCompletionArtifacts;

pub(super) async fn run_completion_drawing(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    planning: MangaPlanningRun,
) -> Result<MangaCompletionArtifacts, AppError> {
    let service = context.service;
    let chapter_id = context.chapter_id;
    let project_id = context.project_id;
    let reference_assets = context.reference_assets.clone();
    let MangaPlanningRun {
        extraction,
        plan_json,
    } = planning;
    let extraction_summary = extraction.extraction_summary;

    let dp_step = service
        .run_page_drawing_prompt_step(DrawingPromptStepInput {
            chapter_id,
            project_id,
            prompt_renderer: context.prompt_renderer,
            vars: &context.variables,
            task_id: context.task_id,
            workflow_run_id: context.workflow_run_id,
            source_metadata: &context.source_metadata,
            reference_assets_metadata: &context.reference_assets_metadata,
            prompt_fingerprint: &context.prompt_fingerprint,
            checkpoint,
            chapter_content: &context.chapter_content,
            source_spans: &context.source_spans,
            reference_assets: &reference_assets,
            plan_json: &plan_json,
            extraction_summary: &extraction_summary,
            restore_from_checkpoint: context.should_restore_stage(7),
        })
        .await?;
    let page_traces = dp_step.page_traces;
    let dp_json = dp_step.json;
    validate_page_drawing_prompts(&dp_json, "manga_page_drawing_prompt")?;
    checkpoint_insert(
        checkpoint,
        "dp_out",
        json!(serde_json::to_string(&dp_json)?),
    );
    checkpoint_insert(checkpoint, "dp_json", dp_json.clone());
    service
        .save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "drawing_prompt",
            current: 7,
            message: "已完成整页漫画提示词编排",
            task_id: context.task_id,
            source_metadata: &context.source_metadata,
            reference_assets_metadata: &context.reference_assets_metadata,
            prompt_fingerprint: &context.prompt_fingerprint,
            checkpoint,
        })
        .await?;
    service
        .ensure_pipeline_not_cancelled(project_id, chapter_id)
        .await?;
    service
        .set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "persisting",
            current: 7,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在写入整页漫画提示词",
            task_id: context.task_id,
            allow_cancel_override: false,
        })
        .await?;

    Ok(MangaCompletionArtifacts {
        plan_json,
        dp_json,
        page_traces,
    })
}
