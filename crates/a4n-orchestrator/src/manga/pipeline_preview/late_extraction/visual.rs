use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS, MangaContextManifestInput,
    MangaContextPreviousOutput, manga_visual_anchor_extraction_user_message,
    validate_manga_visual_anchor_source_span_ids,
};
use super::super::call::{build_manga_preview_call, push_blocked_after};
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_visual(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    events_out: &str,
    events_json: &Value,
    reference_assets: &Value,
) -> Result<Option<(String, Value)>, AppError> {
    let visual_call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_visual_anchor_extraction",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_visual_anchor_extraction",
            context.chapter_id
        ),
        call_order: 3,
        restore_from_checkpoint: context.should_restore_stage(3),
        manifest: MangaContextManifestInput {
            prompt_renderer: context.prompt_renderer,
            project_id: context.project_id,
            variables: &context.variables,
            chapter_content: &context.chapter_content,
            source_spans: &context.source_spans,
            prompt_names: &["manga_visual_anchor_extraction"],
            previous_outputs: &[MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_SHOOTING_SCRIPT_EVENTS,
                prompt_name: "shooting_script_projection",
                parsed_output: &json!({ "events": events_json }),
                raw_output: Some(events_out),
            }],
            reference_assets: Some(reference_assets),
            dynamic_user_content: manga_visual_anchor_extraction_user_message(
                events_json.clone(),
                &context.source_spans,
            )?,
        },
        temperature: 0.3,
    })
    .await?;
    let (visual_out, visual_json) = match visual_call {
        MangaPreviewCall::Restored {
            response_text,
            parsed_output,
        } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_visual_anchor_extraction",
                "漫画 Step 3：视觉锚点抽取",
                "manga_visual_anchor_extraction",
            ));
            (response_text, parsed_output)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_visual_anchor_extraction",
                "漫画 Step 3：视觉锚点抽取",
                "manga_visual_anchor_extraction",
                None,
                *request,
            ));
            push_blocked_after(steps, "manga_item_extraction");
            return Ok(None);
        }
    };
    validate_manga_visual_anchor_source_span_ids(&visual_json, &context.source_spans)?;

    Ok(Some((visual_out, visual_json)))
}
