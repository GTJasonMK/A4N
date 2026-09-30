use a4n_domain::manga::{MangaPageAdaptationV1, MangaPageContentV1, MangaPageSlotsV1, MangaPageV4};
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::adaptation_projection::manga_page_adaptation_source;
use super::super::super::json::{
    page_drawing_prompt_from_response, parse_manga_page_adaptation, parse_manga_page_storyboard,
};
use super::super::super::prompt::{
    manga_page_adaptation_user_message, manga_page_drawing_prompt_pipeline_user_message,
    manga_page_storyboard_user_message,
};
use super::super::super::state::manga_page_pipeline_call_order;
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};

pub(super) async fn preview_page_pipeline(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    slots: &MangaPageSlotsV1,
    extraction_summary: &Value,
) -> Result<(), AppError> {
    let total_pages = slots.pages.len();
    for (index, slot) in slots.pages.iter().enumerate() {
        let page_number = slot.page_number;
        let content = &slot.content;
        let previous_content = index.checked_sub(1).and_then(|i| slots.pages.get(i));
        let next_content = slots.pages.get(index + 1);

        let adaptation = match preview_page_adaptation(
            context,
            steps,
            index,
            page_number,
            content,
            previous_content.map(|page| &page.content),
            next_content.map(|page| &page.content),
            total_pages,
        )
        .await?
        {
            Some(adaptation) => adaptation,
            None => continue,
        };
        if context.stop_after
            == Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation)
        {
            continue;
        }

        let storyboard = match preview_page_storyboard(
            context,
            steps,
            index,
            page_number,
            content,
            &adaptation,
            previous_content.map(|page| &page.content),
            next_content.map(|page| &page.content),
            extraction_summary,
            total_pages,
        )
        .await?
        {
            Some(storyboard) => storyboard,
            None => continue,
        };
        if matches!(
            context.stop_after,
            Some(
                a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard
                    | a4n_api_contract::manga::MangaPipelineStopAfter::PagePlanning
            )
        ) {
            continue;
        }
        preview_page_drawing_prompt(
            context,
            steps,
            index,
            page_number,
            content,
            &adaptation,
            &storyboard,
            extraction_summary,
        )
        .await?;
    }
    Ok(())
}

async fn preview_page_adaptation(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    index: usize,
    page_number: u32,
    content: &MangaPageContentV1,
    previous_content: Option<&MangaPageContentV1>,
    next_content: Option<&MangaPageContentV1>,
    total_pages: usize,
) -> Result<Option<MangaPageAdaptationV1>, AppError> {
    let key = format!("manga_page_adaptation_page_{page_number}");
    let title = format!("漫画 Step 7：第 {page_number} 页改编计划");
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_adaptation_plan",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:page:{}:adaptation",
            context.chapter_id, page_number
        ),
        call_order: manga_page_pipeline_call_order(index, 0)?,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_adaptation_user_message(
            page_number,
            total_pages,
            content,
            manga_page_adaptation_source(&context.adaptation, content),
            previous_content,
            next_content,
        )?,
        temperature: 0.3,
    })
    .await?;
    match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                &key,
                &title,
                "manga_page_adaptation_plan",
            ));
            parse_manga_page_adaptation(&parsed_output, content, &title).map(Some)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                &key,
                &title,
                "manga_page_adaptation_plan",
                None,
                *request,
            ));
            Ok(None)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn preview_page_storyboard(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    index: usize,
    page_number: u32,
    content: &MangaPageContentV1,
    adaptation: &MangaPageAdaptationV1,
    previous_content: Option<&MangaPageContentV1>,
    next_content: Option<&MangaPageContentV1>,
    extraction_summary: &Value,
    total_pages: usize,
) -> Result<Option<MangaPageV4>, AppError> {
    let key = format!("manga_page_storyboard_page_{page_number}");
    let title = format!("漫画 Step 8：第 {page_number} 页分镜");
    let adaptation_beats = manga_page_adaptation_source(&context.adaptation, content);
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_storyboard",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:page:{}:storyboard",
            context.chapter_id, page_number
        ),
        call_order: manga_page_pipeline_call_order(index, 1)?,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_storyboard_user_message(
            page_number,
            total_pages,
            content,
            adaptation,
            previous_content,
            next_content,
            adaptation_beats,
            extraction_summary,
            &context.reference_assets,
        )?,
        temperature: 0.3,
    })
    .await?;
    match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                &key,
                &title,
                "manga_page_storyboard",
            ));
            parse_manga_page_storyboard(&parsed_output, page_number, content, adaptation, &title)
                .map(Some)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                &key,
                &title,
                "manga_page_storyboard",
                None,
                *request,
            ));
            Ok(None)
        }
    }
}

async fn preview_page_drawing_prompt(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    index: usize,
    page_number: u32,
    content: &MangaPageContentV1,
    adaptation: &MangaPageAdaptationV1,
    storyboard: &MangaPageV4,
    extraction_summary: &Value,
) -> Result<(), AppError> {
    let key = format!("manga_page_drawing_prompt_page_{page_number}");
    let title = format!("漫画 Step 9：第 {page_number} 页绘图提示词");
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_drawing_prompt",
        workflow_run_id: context.workflow_run_id,
        call_key: format!(
            "manga_pipeline:{}:manga_page_drawing_prompt:page:{}",
            context.chapter_id, page_number
        ),
        call_order: manga_page_pipeline_call_order(index, 2)?,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_drawing_prompt_pipeline_user_message(
            page_number,
            storyboard,
            content,
            adaptation,
            extraction_summary,
            &context.reference_assets,
        )?,
        temperature: 0.5,
    })
    .await?;
    match call {
        MangaPreviewCall::Restored { parsed_output } => {
            page_drawing_prompt_from_response(
                &parsed_output,
                page_number,
                &format!("manga_page_drawing_prompt.page_{page_number}"),
            )?;
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                &key,
                &title,
                "manga_page_drawing_prompt",
            ));
        }
        MangaPreviewCall::Ready(request) => steps.push(MangaPipelineRequestPreviewStep::ready(
            &key,
            &title,
            "manga_page_drawing_prompt",
            None,
            *request,
        )),
    }
    Ok(())
}
