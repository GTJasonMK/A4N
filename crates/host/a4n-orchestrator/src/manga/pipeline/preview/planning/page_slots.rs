use a4n_domain::manga::MangaPageSlotsV1;
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::adaptation_projection::manga_page_slots_source;
use super::super::super::json::{
    parse_manga_page_content, parse_manga_page_outline, parse_manga_page_slots,
};
use super::super::super::prompt::{MangaPageContentMessages, manga_page_outline_user_message};
use super::super::super::state::MangaPagePlanningStep;
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_slots(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction_summary: &Value,
) -> Result<Option<MangaPageSlotsV1>, AppError> {
    let count = context
        .adaptation
        .plan
        .validate_manga_page_count()
        .map_err(AppError::InvalidArgument)? as usize;
    let outline = preview_call(
        context,
        steps,
        MangaPagePlanningStep::Outline,
        manga_page_outline_user_message(
            extraction_summary.clone(),
            manga_page_slots_source(&context.adaptation),
            &context.source_markers,
        )?,
    )
    .await?
    .map(|value| parse_manga_page_outline(&value, &context.adaptation, &context.source_spans))
    .transpose()?;
    let mut drafts = Vec::with_capacity(count);
    for index in 0..count {
        let step = MangaPagePlanningStep::Content(index);
        let Some(outline) = outline.as_ref() else {
            steps.push(MangaPipelineRequestPreviewStep {
                key: step.key(),
                title: step.title(),
                prompt_name: Some(step.prompt_name().into()),
                state: "waiting_dependencies".into(),
                note: Some("等待轻量页纲完成后，使用本页真实分配构建请求".into()),
                request: None,
            });
            continue;
        };
        let messages = MangaPageContentMessages {
            outline,
            extraction_summary,
            adaptation: &context.adaptation,
            source_spans: &context.source_spans,
            chapter_content: &context.chapter_content,
            source_markers: &context.source_markers,
        };
        if let Some(value) = preview_call(context, steps, step, messages.build(index)?).await? {
            drafts.push(parse_manga_page_content(&value, outline, index)?);
        }
    }
    if drafts.len() != count {
        return Ok(None);
    }
    let outline = outline.expect("drafts require a validated outline");
    let slots = outline
        .assemble(drafts)
        .map_err(AppError::InvalidArgument)?;
    parse_manga_page_slots(
        &serde_json::to_value(slots)?,
        &context.adaptation,
        &context.source_spans,
        "manga_page_slots",
    )
    .map(Some)
}

async fn preview_call(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    step: MangaPagePlanningStep,
    user_content: a4n_llm_context::LlmContextManifest,
) -> Result<Option<Value>, AppError> {
    match build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: step.prompt_name(),
        workflow_run_id: context.workflow_run_id,
        call_key: step.call_key(context.chapter_id),
        call_order: step.call_order()?,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content,
        temperature: 0.3,
    })
    .await?
    {
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                &step.key(),
                &step.title(),
                step.prompt_name(),
                None,
                *request,
            ));
            Ok(None)
        }
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                &step.key(),
                &step.title(),
                step.prompt_name(),
            ));
            Ok(Some(parsed_output))
        }
    }
}
