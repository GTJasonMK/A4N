use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::pipeline_json::parse_manga_page_plan;
use super::super::super::pipeline_prompt::{
    MangaContextManifestInput, manga_page_planning_user_message,
};
use super::super::super::shooting_script_projection::manga_page_planning_source;
use super::super::call::{build_manga_preview_call, push_blocked_after};
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_plan(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction_summary: &Value,
    reference_assets: &Value,
) -> Result<Option<Value>, AppError> {
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_planning",
        workflow_run_id: context.workflow_run_id,
        call_key: format!("manga_pipeline:{}:manga_page_planning", context.chapter_id),
        call_order: 6,
        restore_from_checkpoint: context.should_restore_stage(6),
        manifest: MangaContextManifestInput {
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            variables: &context.variables,
            chapter_content: &context.chapter_content,
            source_spans: &context.source_spans,
            prompt_names: &["manga_page_planning"],
            previous_outputs: &[],
            reference_assets: Some(reference_assets),
            dynamic_user_content: manga_page_planning_user_message(
                extraction_summary.clone(),
                manga_page_planning_source(
                    context.shooting_script.record.id,
                    context.shooting_script.record.revision,
                    &context.shooting_script.script,
                ),
            )?,
        },
        temperature: 0.3,
    })
    .await?;
    let raw_plan_json = match call {
        MangaPreviewCall::Restored { parsed_output, .. } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_page_planning",
                "漫画 Step 6：分页规划",
                "manga_page_planning",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_page_planning",
                "漫画 Step 6：分页规划",
                "manga_page_planning",
                None,
                *request,
            ));
            push_blocked_after(steps, "manga_page_drawing_prompt");
            return Ok(None);
        }
    };
    let plan_json = serde_json::to_value(parse_manga_page_plan(
        &raw_plan_json,
        &context.shooting_script,
        "manga_page_planning",
    )?)?;
    Ok(Some(plan_json))
}
