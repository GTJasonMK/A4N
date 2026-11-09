use std::collections::HashMap;

use a4n_domain::manga::{
    MangaPageContentV1, MangaPagePromptRecord, MangaPageSlotsV1, MangaPipelineArtifactStage,
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use futures::{StreamExt, stream};
use serde_json::Value;

use crate::manga::layout_rules::manga_page_pipeline_layout;

use super::super::persist::{MangaPipelinePersistInput, pipeline_page_draft};
use super::contracts::MangaRunContext;
use super::page::{
    MANGA_PAGE_PIPELINE_CONCURRENCY, MangaPagePipelineProgress, manga_page_pipeline_total_steps,
    persist_page_pipeline_artifact_delta, prepare_manga_page_work, process_manga_page,
};
use crate::manga::validate_manga_page_scope;

pub(super) async fn complete_pipeline(
    context: &MangaRunContext<'_>,
    extraction_summary: &Value,
    mut slots: MangaPageSlotsV1,
) -> Result<Vec<MangaPagePromptRecord>, AppError> {
    validate_manga_page_scope(context.page_number, context.stop_after)?;
    let artifacts = context
        .service
        .manga_pipeline_artifacts_by_chapter(context.project_id, context.chapter_id)
        .await?;
    for slot in &mut slots.pages {
        if let Some(artifact) = artifacts.iter().find(|artifact| {
            artifact.stage == MangaPipelineArtifactStage::PageContent
                && artifact.page_number == Some(slot.page_number)
                && !artifact.stale
        }) {
            slot.content = serde_json::from_value::<MangaPageContentV1>(artifact.payload.clone())?;
            slot.content.validate().map_err(AppError::InvalidArgument)?;
        }
    }
    slots.validate().map_err(AppError::InvalidArgument)?;
    let total_pages = slots.pages.len();
    let page_numbers = slots
        .pages
        .iter()
        .map(|slot| slot.page_number)
        .collect::<Vec<_>>();
    let selected_page_indexes = page_indexes_to_process(&page_numbers, context.page_number)?;
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
    let mut drafts = Vec::with_capacity(selected_page_indexes.len());
    let mut projection_deltas = Vec::with_capacity(selected_page_indexes.len());
    for index in 0..total_pages {
        let existing = existing_by_page
            .get(
                &i32::try_from(slots.pages[index].page_number)
                    .map_err(|_| AppError::InvalidArgument("漫画页码超出 i32 范围".into()))?,
            )
            .cloned();
        let existing_layout = existing
            .as_ref()
            .map(manga_page_pipeline_layout)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        let work = prepare_manga_page_work(
            context,
            &slots,
            index,
            extraction_summary,
            &artifacts,
            existing,
        )?;
        if selected_page_indexes.binary_search(&index).is_ok() {
            let old_layout = work.row.layout.clone();
            let draft = pipeline_page_draft(
                &persist,
                work.slot.page_number,
                work.pipeline.clone(),
                Vec::new(),
                existing_layout.as_ref(),
            )?;
            projection_deltas.push((old_layout, draft.layout.clone()));
            drafts.push(draft);
        }
        works.push(work);
    }
    let rows = context
        .service
        .sync_generated_page_prompts(context.project_id, context.chapter_id, drafts)
        .await?;
    if rows.len() != selected_page_indexes.len() {
        return Err(AppError::InvalidState {
            current: format!(
                "selected_pages={}, saved_rows={}",
                selected_page_indexes.len(),
                rows.len()
            ),
            operation: "同步漫画页面槽位".to_string(),
        });
    }
    for ((index, row), (old_layout, new_layout)) in selected_page_indexes
        .iter()
        .copied()
        .zip(rows)
        .zip(projection_deltas.iter())
    {
        let content_stage = artifacts.iter().find(|artifact| {
            artifact.stage == MangaPipelineArtifactStage::PageContent
                && artifact.page_number == Some(works[index].slot.page_number)
        });
        persist_page_pipeline_artifact_delta(
            context,
            works[index].slot.page_number,
            old_layout.as_ref(),
            new_layout.as_ref(),
            content_stage
                .is_none()
                .then_some(MangaPipelineArtifactStage::PageContent),
        )
        .await?;
        works[index].row = row;
    }

    let total_steps =
        manga_page_pipeline_total_steps(selected_page_indexes.len(), context.stop_after)?;
    if matches!(
        context.stop_after,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots)
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
    let results = stream::iter(
        works
            .into_iter()
            .enumerate()
            .filter_map(|(index, work)| {
                selected_page_indexes
                    .binary_search(&index)
                    .ok()
                    .map(|_| work)
            })
            .map(|work| {
                process_manga_page(
                    context,
                    &persist,
                    work,
                    extraction_summary,
                    &progress,
                    total_pages,
                )
            }),
    )
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
    if rows.len() != selected_page_indexes.len() {
        return Err(AppError::InvalidState {
            current: format!(
                "expected_rows={}, actual_rows={}",
                selected_page_indexes.len(),
                rows.len()
            ),
            operation: "完成漫画页面流水线".to_string(),
        });
    }
    let (stage, message) = completed_page_stage(
        context.stop_after,
        selected_page_indexes.len(),
        context.page_number,
    );
    complete_at_stage(context, rows, stage, total_steps, message).await
}

async fn complete_at_stage(
    context: &MangaRunContext<'_>,
    rows: Vec<MangaPagePromptRecord>,
    stage: &str,
    total_steps: i32,
    message: String,
) -> Result<Vec<MangaPagePromptRecord>, AppError> {
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
        stage,
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
    page_number: Option<u32>,
) -> (&'static str, String) {
    let scope = page_number
        .map(|page_number| format!("第 {page_number} 页"))
        .unwrap_or_else(|| format!("{page_count} 页"));
    match stop_after {
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation) => {
            ("adaptation", format!("{scope}页面改编计划已完成"))
        }
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard) => {
            ("storyboard", format!("{scope}分镜已完成"))
        }
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::ReferenceDiscovery) => {
            ("reference_discovery", format!("{scope}参考图发现已完成"))
        }
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::DrawingPrompt) => {
            ("drawing_prompt", format!("{scope}整页提示词已完成"))
        }
        _ if page_number.is_some() => ("completed", format!("{scope}漫画流水线已完成")),
        _ => (
            "completed",
            format!("漫画页面流水线完成，共生成 {page_count} 页图片"),
        ),
    }
}

fn page_indexes_to_process(
    page_numbers: &[u32],
    page_number: Option<u32>,
) -> Result<Vec<usize>, AppError> {
    if let Some(page_number) = page_number {
        return page_numbers
            .iter()
            .position(|candidate| *candidate == page_number)
            .map(|index| vec![index])
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "漫画第 {page_number} 页不存在，无法启动单页流水线"
                ))
            });
    }
    Ok((0..page_numbers.len()).collect())
}

#[cfg(test)]
mod tests {
    use super::page_indexes_to_process;

    #[test]
    fn page_scope_selects_only_the_requested_slot() {
        assert_eq!(page_indexes_to_process(&[1, 2, 3], Some(2)).unwrap(), [1]);
    }

    #[test]
    fn missing_page_scope_is_rejected() {
        assert!(page_indexes_to_process(&[1, 2, 3], Some(4)).is_err());
    }

    #[test]
    fn absent_page_scope_keeps_full_chapter_behavior() {
        assert_eq!(
            page_indexes_to_process(&[1, 2, 3], None).unwrap(),
            [0, 1, 2]
        );
    }
}
