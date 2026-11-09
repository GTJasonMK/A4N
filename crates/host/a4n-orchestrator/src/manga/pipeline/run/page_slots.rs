mod progress;

use a4n_domain::manga::{MangaPageContentDraftV1, MangaPageSlotsV1, MangaPipelineArtifactStage};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use futures::{StreamExt, stream::FuturesUnordered};
use serde_json::Value;

use super::super::super::adaptation_projection::manga_page_slots_source;
use super::super::json::{
    parse_manga_page_content, parse_manga_page_outline, parse_manga_page_slots,
    validate_manga_page_slots,
};
use super::super::prompt::{MangaPageContentMessages, manga_page_outline_user_message};
use super::super::state::MangaPagePlanningStep;
use super::contracts::{MangaRunContext, manga_stage_output_exists_error};
use super::page::{MANGA_PAGE_PIPELINE_CONCURRENCY, manga_page_pipeline_total_steps};
use progress::PagePlanningProgress;

pub(super) async fn run_page_slots(
    context: &MangaRunContext<'_>,
    extraction_summary: &Value,
) -> Result<MangaPageSlotsV1, AppError> {
    context.check_cancelled()?;
    let user_content = manga_page_outline_user_message(
        extraction_summary.clone(),
        manga_page_slots_source(&context.adaptation),
        &context.source_markers,
    )?;
    let input_fingerprint =
        context.artifact_input_fingerprint(MangaPipelineArtifactStage::PageSlots, &user_content)?;
    let total_pages = context
        .adaptation
        .plan
        .validate_manga_page_count()
        .map_err(AppError::InvalidArgument)? as usize;
    let total = manga_page_pipeline_total_steps(total_pages, total_pages, context.stop_after)?;
    let progress = PagePlanningProgress::new(context, total);
    if let Some(artifact) = context
        .service
        .manga_pipeline_artifact(
            context.project_id,
            context.chapter_id,
            MangaPipelineArtifactStage::PageSlots,
            None,
        )
        .await?
    {
        if artifact.stale
            || artifact.input_fingerprint != input_fingerprint
            || (context.run_stage == Some(MangaPipelineArtifactStage::PageSlots)
                && !context.resume_from_checkpoint)
        {
            return Err(manga_stage_output_exists_error(
                MangaPipelineArtifactStage::PageSlots,
                None,
            ));
        }
        let slots = parse_manga_page_slots(
            &artifact.payload,
            &context.adaptation,
            &context.source_spans,
            "manga_page_slots",
        )?;
        progress
            .report(
                None,
                "已复用完整分页结果",
                super::super::state::manga_page_planning_total_steps(total_pages)?,
                ProgressPhaseKind::Save,
            )
            .await?;
        return Ok(slots);
    }
    let outline = progress
        .run_call(MangaPagePlanningStep::Outline, user_content, |value| {
            parse_manga_page_outline(value, &context.adaptation, &context.source_spans)
        })
        .await?;
    let messages = MangaPageContentMessages {
        outline: &outline,
        extraction_summary,
        adaptation: &context.adaptation,
        source_spans: &context.source_spans,
        chapter_content: &context.chapter_content,
        source_markers: &context.source_markers,
    };
    let mut pending = 0..total_pages;
    let mut running = FuturesUnordered::new();
    for index in pending.by_ref().take(MANGA_PAGE_PIPELINE_CONCURRENCY) {
        running.push(run_page(&progress, &messages, index));
    }
    let mut drafts = vec![None; total_pages];
    let mut first_error = None;
    while let Some((index, result)) = running.next().await {
        match result {
            Ok(draft) => drafts[index] = Some(draft),
            Err(error) => {
                if !matches!(error, AppError::Cancelled) {
                    tracing::error!(page_number = index + 1, error = %error, "漫画页面内容规划失败");
                }
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        // Drain in-flight calls after a failure so their completed checkpoints survive.
        if first_error.is_none()
            && let Some(next) = pending.next()
        {
            running.push(run_page(&progress, &messages, next));
        }
    }
    context.check_cancelled()?;
    if let Some(error) = first_error {
        return Err(error);
    }
    let drafts = drafts
        .into_iter()
        .enumerate()
        .map(|(index, draft)| {
            draft.ok_or_else(|| AppError::InvalidState {
                current: format!("page_content_missing:{}", index + 1),
                operation: "组装完整分页".into(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let slots = outline
        .assemble(drafts)
        .map_err(AppError::InvalidArgument)?;
    validate_manga_page_slots(
        &slots,
        &context.adaptation,
        &context.source_spans,
        "manga_page_slots",
    )?;
    context.check_cancelled()?;
    let (current_adaptation, current_spans) = context
        .service
        .current_manga_adaptation_and_source_spans(context.project_id, context.chapter_id)
        .await?;
    if current_adaptation.record.reference() != context.adaptation.record.reference()
        || current_spans != context.source_spans
    {
        return Err(AppError::InvalidState {
            current: "manga_page_planning_inputs_changed".into(),
            operation: "分页期间来源或已采用改编发生变化，请重新规划".into(),
        });
    }
    context
        .persist_artifact(
            MangaPipelineArtifactStage::PageSlots,
            None,
            input_fingerprint,
            serde_json::to_value(&slots)?,
        )
        .await?;
    progress
        .report(None, "已校验并保存完整分页结果", 1, ProgressPhaseKind::Save)
        .await?;
    Ok(slots)
}

async fn run_page(
    progress: &PagePlanningProgress<'_, '_>,
    messages: &MangaPageContentMessages<'_>,
    index: usize,
) -> (usize, Result<MangaPageContentDraftV1, AppError>) {
    let result = async {
        progress
            .run_call(
                MangaPagePlanningStep::Content(index),
                messages.build(index)?,
                |value| parse_manga_page_content(value, messages.outline, index),
            )
            .await
    }
    .await;
    (index, result)
}
