use a4n_domain::manga::MangaPageSlotsV1;
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::adaptation_projection::manga_page_slots_source;
use super::super::super::json::parse_manga_page_slots;
use super::super::super::prompt::manga_page_slots_user_message;
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_slots(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction_summary: &Value,
) -> Result<Option<MangaPageSlotsV1>, AppError> {
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_slots",
        workflow_run_id: context.workflow_run_id,
        call_key: format!("manga_pipeline:{}:page_slots", context.chapter_id),
        call_order: 6,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_slots_user_message(
            extraction_summary.clone(),
            manga_page_slots_source(&context.adaptation),
            &context.source_spans,
            &context.chapter_content,
            &context.source_markers,
        )?,
        temperature: 0.3,
    })
    .await?;
    let raw_slots = match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                "manga_page_slots",
                "页数与每页内容",
                "manga_page_slots",
            ));
            parsed_output
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                "manga_page_slots",
                "页数与每页内容",
                "manga_page_slots",
                None,
                *request,
            ));
            return Ok(None);
        }
    };
    parse_manga_page_slots(
        &raw_slots,
        &context.adaptation,
        &context.source_spans,
        "manga_page_slots",
    )
    .map(Some)
}
