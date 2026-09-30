use std::collections::HashMap;

use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use futures::{StreamExt, stream};

use super::super::persist::{MangaPipelinePersistInput, pipeline_page_draft};
use super::contracts::{MangaExtractionRun, MangaRunContext};
use super::page::{
    MANGA_PAGE_PIPELINE_CONCURRENCY, MangaPagePipelineProgress, manga_page_pipeline_total_steps,
    prepare_manga_page_work, process_manga_page,
};
use super::page_slots::MangaPageSlotsRun;

pub(super) async fn complete_pipeline(
    context: &MangaRunContext<'_>,
    extraction: MangaExtractionRun,
    slots_run: MangaPageSlotsRun,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    let total_pages = slots_run.slots.pages.len();
    let extraction_summary = extraction.extraction_summary;
    let persist = MangaPipelinePersistInput {
        chapter_id: context.chapter_id,
        project_id: context.project_id,
        source_metadata: &context.source_metadata,
        reference_assets_metadata: &context.reference_assets_metadata,
        prompt_fingerprint: &context.prompt_fingerprint,
        page_plan_ref: None,
    };
    let existing = context
        .service
        .manga_page_prompts_by_chapter(context.project_id, context.chapter_id)
        .await?;
    let existing_by_page = existing
        .into_iter()
        .map(|row| (row.page_number, row))
        .collect::<HashMap<_, _>>();

    let mut works = Vec::with_capacity(total_pages);
    let mut drafts = Vec::with_capacity(total_pages);
    for index in 0..total_pages {
        let existing = existing_by_page
            .get(
                &i32::try_from(slots_run.slots.pages[index].page_number)
                    .map_err(|_| AppError::InvalidArgument("漫画页码超出 i32 范围".into()))?,
            )
            .cloned();
        let existing_layout = existing
            .as_ref()
            .and_then(|row| a4n_manga_core::manga_page_pipeline_layout(row).ok());
        let work = prepare_manga_page_work(
            context,
            &slots_run.slots,
            index,
            &extraction_summary,
            existing,
        )?;
        drafts.push(pipeline_page_draft(
            &persist,
            work.slot.page_number,
            work.pipeline.clone(),
            Vec::new(),
            existing_layout.as_ref(),
        )?);
        works.push(work);
    }
    let rows = context
        .service
        .sync_generated_page_prompts(context.project_id, context.chapter_id, drafts)
        .await?;
    if rows.len() != works.len() {
        return Err(AppError::InvalidState {
            current: format!("page_slots={}, saved_rows={}", works.len(), rows.len()),
            operation: "同步漫画页面槽位".to_string(),
        });
    }
    for (work, row) in works.iter_mut().zip(rows) {
        work.row = row;
    }

    let total_steps = manga_page_pipeline_total_steps(total_pages, context.stop_after)?;
    if matches!(
        context.stop_after,
        Some(
            a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots
                | a4n_api_contract::manga::MangaPipelineStopAfter::PageContent
        )
    ) {
        return complete_at_stage(
            context,
            works.into_iter().map(|work| work.row).collect(),
            "page_content",
            total_steps,
            "已确定页面数量和每页内容".to_string(),
        )
        .await;
    }

    let progress = MangaPagePipelineProgress::new(total_steps);
    let results = stream::iter(works.into_iter().map(|work| {
        process_manga_page(
            context,
            &persist,
            work,
            &extraction_summary,
            &progress,
            total_pages,
        )
    }))
    .buffer_unordered(MANGA_PAGE_PIPELINE_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    let mut rows = Vec::with_capacity(results.len());
    let mut first_error = None;
    for result in results {
        match result {
            Ok(row) => rows.push(row),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    rows.sort_by_key(|row| row.page_number);
    if rows.len() != total_pages {
        return Err(AppError::InvalidState {
            current: format!("expected_rows={total_pages}, actual_rows={}", rows.len()),
            operation: "完成漫画页面流水线".to_string(),
        });
    }
    let (stage, message) = completed_page_stage(context.stop_after, total_pages);
    complete_at_stage(context, rows, stage, total_steps, message).await
}

async fn complete_at_stage(
    context: &MangaRunContext<'_>,
    rows: Vec<chapter_manga_prompt::Model>,
    stage: &str,
    total_steps: i32,
    message: String,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    context
        .service
        .record_manga_pipeline_progress_with_total(
            context.workflow_run_id,
            context.task_id,
            stage,
            total_steps,
            total_steps,
            &message,
        )
        .await?;
    context.publish_progress(
        ProgressPhaseKind::Complete,
        total_steps,
        total_steps,
        &message,
        "阶段",
        None,
        None,
        None,
    );
    Ok(rows)
}

fn completed_page_stage(
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    page_count: usize,
) -> (&'static str, String) {
    match stop_after {
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation) => {
            ("adaptation", format!("{page_count} 页页面改编计划已完成"))
        }
        Some(
            a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard
            | a4n_api_contract::manga::MangaPipelineStopAfter::PagePlanning,
        ) => ("storyboard", format!("{page_count} 页分镜已完成")),
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::DrawingPrompt) => {
            ("drawing_prompt", format!("{page_count} 页整页提示词已完成"))
        }
        _ => (
            "completed",
            format!("漫画页面流水线完成，共生成 {page_count} 页图片"),
        ),
    }
}
