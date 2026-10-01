use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU32, Ordering},
};

use a4n_api_contract::image_generation::GeneratePageImageInput;
use a4n_db::manga::MANGA_PAGE_PROMPT_TYPE;
use a4n_domain::{
    manga::{
        MANGA_PAGE_PIPELINE_STATE_V1, MangaPageAdaptationV1, MangaPageContentV1,
        MangaPageImageArtifactV1, MangaPagePipelineErrorV1, MangaPagePipelineStage,
        MangaPagePipelineStateV1, MangaPagePipelineStatus, MangaPagePromptRecord, MangaPageSlotV1,
        MangaPageSlotsV1,
    },
    source_text_hash,
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::manga::layout_rules::manga_page_pipeline_layout;

use super::super::super::adaptation_projection::manga_page_adaptation_source;
use super::super::{
    json::{parse_manga_page_adaptation, parse_manga_page_storyboard},
    persist::{MangaPipelinePersistInput, pipeline_page_draft},
    prompt::{
        MangaPromptGenerationTrace, MangaRenderTraceInput, manga_page_adaptation_user_message,
        manga_page_storyboard_user_message,
    },
    state::{MANGA_PIPELINE_TOTAL_STEPS, manga_page_pipeline_call_order},
};
use super::{contracts::MangaRunContext, drawing_prompt::SinglePageDrawingPromptInput};

pub(super) const MANGA_PAGE_PIPELINE_CONCURRENCY: usize = 2;
const PAGE_PIPELINE_STAGES: [MangaPagePipelineStage; 4] = [
    MangaPagePipelineStage::Adaptation,
    MangaPagePipelineStage::Storyboard,
    MangaPagePipelineStage::DrawingPrompt,
    MangaPagePipelineStage::Image,
];

pub(super) struct MangaPageWork {
    pub(super) slot: MangaPageSlotV1,
    pub(super) page_index: usize,
    pub(super) previous_content: Option<MangaPageContentV1>,
    pub(super) next_content: Option<MangaPageContentV1>,
    pub(super) pipeline: MangaPagePipelineStateV1,
    pub(super) row: MangaPagePromptRecord,
}

pub(super) struct MangaPagePipelineProgress {
    completed: AtomicU32,
    total: i32,
    lock: Mutex<()>,
}

impl MangaPagePipelineProgress {
    pub(super) fn new(total: i32) -> Self {
        Self {
            completed: AtomicU32::new(0),
            total,
            lock: Mutex::new(()),
        }
    }

    async fn advance(
        &self,
        context: &MangaRunContext<'_>,
        page_number: u32,
        total_pages: usize,
        stage: MangaPagePipelineStage,
        message: &str,
    ) -> Result<(), AppError> {
        let _guard = self.lock.lock().await;
        let completed = self
            .completed
            .load(Ordering::Acquire)
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidArgument("漫画页面进度计数溢出".into()))?;
        let current =
            MANGA_PIPELINE_TOTAL_STEPS
                .checked_add(i32::try_from(completed).map_err(|_| {
                    AppError::InvalidArgument("漫画页面进度计数超出 i32 范围".into())
                })?)
                .ok_or_else(|| AppError::InvalidArgument("漫画页面进度超出 i32 范围".into()))?;
        let label = format!("第 {page_number}/{total_pages} 页：{message}");
        context
            .service
            .record_manga_pipeline_progress_with_total(
                context.workflow_run_id,
                context.task_id,
                stage.key(),
                current,
                self.total,
                &label,
            )
            .await?;
        self.completed.store(completed, Ordering::Release);
        context.publish_progress(
            if stage == MangaPagePipelineStage::Image {
                ProgressPhaseKind::AssetGeneration
            } else {
                ProgressPhaseKind::StructuredOutput
            },
            current,
            self.total,
            &label,
            "页",
            Some(format!("page-{page_number:04}")),
            Some(u64::from(page_number)),
            u64::try_from(total_pages).ok(),
        );
        Ok(())
    }

    async fn llm_usage(
        &self,
        context: &MangaRunContext<'_>,
        stage: MangaPagePipelineStage,
        trace: &MangaPromptGenerationTrace,
    ) -> Result<(), AppError> {
        let _guard = self.lock.lock().await;
        let current = MANGA_PIPELINE_TOTAL_STEPS
            .checked_add(
                i32::try_from(self.completed.load(Ordering::Acquire)).map_err(|_| {
                    AppError::InvalidArgument("漫画页面进度计数超出 i32 范围".into())
                })?,
            )
            .ok_or_else(|| AppError::InvalidArgument("漫画页面进度超出 i32 范围".into()))?;
        let message = format!(
            "模型请求完成：{}；{}；正在校验结构",
            trace.prompt_name,
            a4n_llm_trace::llm_usage_summary_text(trace.usage.as_ref())
        );
        context
            .service
            .record_manga_pipeline_progress_with_total(
                context.workflow_run_id,
                context.task_id,
                stage.key(),
                current,
                self.total,
                &message,
            )
            .await
    }

    fn subprogress(
        &self,
        context: &MangaRunContext<'_>,
        page_number: u32,
        total_pages: usize,
        message: &str,
    ) {
        let Ok(completed) = i32::try_from(self.completed.load(Ordering::Acquire)) else {
            return;
        };
        let current = MANGA_PIPELINE_TOTAL_STEPS + completed;
        let label = format!("第 {page_number}/{total_pages} 页：{message}");
        context.publish_progress(
            ProgressPhaseKind::AssetGeneration,
            current,
            self.total,
            &label,
            "页",
            Some(format!("page-{page_number:04}")),
            Some(u64::from(page_number)),
            u64::try_from(total_pages).ok(),
        );
    }
}

pub(super) fn manga_page_pipeline_total_steps(
    page_count: usize,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Result<i32, AppError> {
    let stages_per_page = match stop_after {
        Some(
            a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots
            | a4n_api_contract::manga::MangaPipelineStopAfter::PageContent,
        ) => 0,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation) => 1,
        Some(
            a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard
            | a4n_api_contract::manga::MangaPipelineStopAfter::PagePlanning,
        ) => 2,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::DrawingPrompt) => 3,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::Image) | None => {
            i32::try_from(PAGE_PIPELINE_STAGES.len())
                .map_err(|_| AppError::InvalidArgument("漫画页面阶段数超出 i32 范围".into()))?
        }
    };
    let page_steps = i32::try_from(page_count)
        .ok()
        .and_then(|pages| pages.checked_mul(stages_per_page))
        .ok_or_else(|| AppError::InvalidArgument("漫画页面进度总数超出 i32 范围".into()))?;
    MANGA_PIPELINE_TOTAL_STEPS
        .checked_add(page_steps)
        .ok_or_else(|| AppError::InvalidArgument("漫画页面进度总数超出 i32 范围".into()))
}

pub(super) fn prepare_manga_page_work(
    context: &MangaRunContext<'_>,
    slots: &MangaPageSlotsV1,
    page_index: usize,
    extraction_summary: &Value,
    existing_row: Option<MangaPagePromptRecord>,
) -> Result<MangaPageWork, AppError> {
    let slot = slots
        .pages
        .get(page_index)
        .ok_or_else(|| AppError::InvalidArgument(format!("漫画页面索引不存在：{page_index}")))?;
    let previous_content = page_index
        .checked_sub(1)
        .and_then(|index| slots.pages.get(index))
        .map(|page| page.content.clone());
    let next_content = slots
        .pages
        .get(page_index + 1)
        .map(|page| page.content.clone());
    let content_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Content,
        json!({"page_content": slot.content}),
    )?;
    let adaptation_plan = manga_page_adaptation_source(&context.adaptation, &slot.content);
    let adaptation_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Adaptation,
        json!({
            "page_content": slot.content,
            "adaptation_plan": adaptation_plan,
            "previous_page_content": previous_content,
            "next_page_content": next_content,
        }),
    )?;

    let reusable_layout = existing_row.as_ref().and_then(|row| {
        manga_page_pipeline_layout(row).ok().filter(|layout| {
            layout.page_number == slot.page_number
                && layout.source == context.source_metadata
                && layout.prompt_fingerprint == context.prompt_fingerprint
                && layout.pipeline.source_fingerprint == context.source_metadata.content_sha256
                && layout.pipeline.page_content.as_ref() == Some(&slot.content)
                && layout.pipeline.stage_fingerprints.get("content") == Some(&content_fingerprint)
        })
    });
    let mut pipeline = reusable_layout.map_or_else(
        || new_page_pipeline(context, slot, content_fingerprint.clone()),
        |layout| layout.pipeline,
    );

    if pipeline.page_adaptation.is_some()
        && pipeline.stage_fingerprints.get("adaptation") != Some(&adaptation_fingerprint)
    {
        pipeline.page_adaptation = None;
        pipeline.storyboard = None;
        pipeline.drawing_prompt = None;
        pipeline.image = None;
        pipeline.stage_fingerprints.remove("adaptation");
        pipeline.stage_fingerprints.remove("storyboard");
        pipeline.stage_fingerprints.remove("drawing_prompt");
        pipeline.stage_fingerprints.remove("image");
    }
    let storyboard_fingerprint = pipeline
        .page_adaptation
        .as_ref()
        .map(|adaptation| {
            page_stage_fingerprint(
                context,
                MangaPagePipelineStage::Storyboard,
                storyboard_fingerprint_input(
                    context,
                    slot,
                    adaptation,
                    previous_content.as_ref(),
                    next_content.as_ref(),
                    extraction_summary,
                ),
            )
        })
        .transpose()?;
    if pipeline.storyboard.is_some()
        && pipeline.stage_fingerprints.get("storyboard") != storyboard_fingerprint.as_ref()
    {
        pipeline.storyboard = None;
        pipeline.drawing_prompt = None;
        pipeline.image = None;
        pipeline.stage_fingerprints.remove("storyboard");
        pipeline.stage_fingerprints.remove("drawing_prompt");
        pipeline.stage_fingerprints.remove("image");
    }

    let mut manual_prompt_override = false;
    if let (Some(row), Some(drawing_prompt)) =
        (existing_row.as_ref(), pipeline.drawing_prompt.as_mut())
        && !row.image_prompt.trim().is_empty()
        && row.image_prompt != drawing_prompt.image_prompt
    {
        drawing_prompt.image_prompt = row.image_prompt.clone();
        pipeline.image = None;
        pipeline.stage_fingerprints.remove("image");
        manual_prompt_override = true;
    }

    let drawing_fingerprint = pipeline.storyboard.as_ref().map(|storyboard| {
        page_stage_fingerprint(
            context,
            MangaPagePipelineStage::DrawingPrompt,
            json!({
                "page_content": slot.content,
                "page_adaptation": pipeline.page_adaptation,
                "storyboard": storyboard,
                "characters": extraction_summary.get("characters"),
                "reference_assets": context.reference_assets,
            }),
        )
    });
    let drawing_fingerprint = drawing_fingerprint.transpose()?;
    if pipeline.drawing_prompt.is_some()
        && pipeline.stage_fingerprints.get("drawing_prompt") != drawing_fingerprint.as_ref()
        && !manual_prompt_override
    {
        pipeline.drawing_prompt = None;
        pipeline.image = None;
        pipeline.stage_fingerprints.remove("drawing_prompt");
        pipeline.stage_fingerprints.remove("image");
    }

    if let Some(drawing_prompt) = pipeline.drawing_prompt.as_ref() {
        let bindings = pipeline
            .storyboard
            .as_ref()
            .map(|page| {
                super::super::super::reference_assets::page_reference_bindings(
                    page,
                    &context.reference_assets_metadata,
                )
                .map_err(AppError::InvalidArgument)
            })
            .transpose()?
            .unwrap_or_default();
        let image_fingerprint = page_stage_fingerprint(
            context,
            MangaPagePipelineStage::Image,
            json!({
                "drawing_prompt": drawing_prompt,
                "reference_bindings": bindings,
                "image_size": context.image_size,
            }),
        )?;
        let image_matches = pipeline.image.as_ref().is_some_and(|image| {
            Some(image.generated_image_id)
                == existing_row.as_ref().and_then(|row| row.generated_image_id)
                && existing_row
                    .as_ref()
                    .is_some_and(|row| row.status == "completed")
                && pipeline.stage_fingerprints.get("image") == Some(&image_fingerprint)
        });
        if !image_matches {
            pipeline.image = None;
            pipeline.stage_fingerprints.remove("image");
        }
    }

    let image_completed = pipeline.image.is_some();
    let next_stage = if image_completed || pipeline.drawing_prompt.is_some() {
        MangaPagePipelineStage::Image
    } else if pipeline.storyboard.is_some() {
        MangaPagePipelineStage::DrawingPrompt
    } else if pipeline.page_adaptation.is_some() {
        MangaPagePipelineStage::Storyboard
    } else {
        MangaPagePipelineStage::Adaptation
    };
    let next_attempt = existing_row.as_ref().and_then(|row| {
        manga_page_pipeline_layout(row)
            .ok()
            .map(|layout| layout.pipeline.attempt)
    });
    let is_existing_complete = image_completed
        && existing_row
            .as_ref()
            .is_some_and(|row| row.status == "completed");
    pipeline.attempt = if is_existing_complete {
        next_attempt.unwrap_or(1)
    } else {
        next_attempt
            .map(|attempt| {
                attempt.checked_add(1).ok_or_else(|| {
                    AppError::InvalidArgument("漫画页面 attempt 超出 u32 范围".into())
                })
            })
            .transpose()?
            .unwrap_or(1)
    };
    pipeline.stage = next_stage;
    pipeline.status = if is_existing_complete {
        MangaPagePipelineStatus::Completed
    } else {
        MangaPagePipelineStatus::Pending
    };
    pipeline.error = None;
    pipeline
        .stage_fingerprints
        .insert("content".into(), content_fingerprint);

    let page_number_db = i32::try_from(slot.page_number).map_err(|_| {
        AppError::InvalidArgument(format!("漫画页码超出 i32 范围：{}", slot.page_number))
    })?;
    let row = existing_row.unwrap_or_else(|| MangaPagePromptRecord {
        id: 0,
        chapter_id: context.chapter_id,
        project_id: context.project_id,
        page_number: page_number_db,
        prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
        image_prompt: String::new(),
        dialogue_text: None,
        characters_in_frame: None,
        layout: None,
        generated_image_id: None,
        status: "pending".to_string(),
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    });
    Ok(MangaPageWork {
        slot: slot.clone(),
        page_index,
        previous_content,
        next_content,
        pipeline,
        row,
    })
}

fn new_page_pipeline(
    context: &MangaRunContext<'_>,
    slot: &MangaPageSlotV1,
    content_fingerprint: String,
) -> MangaPagePipelineStateV1 {
    MangaPagePipelineStateV1 {
        schema_version: MANGA_PAGE_PIPELINE_STATE_V1.to_string(),
        page_id: format!("page-{:04}", slot.page_number),
        status: MangaPagePipelineStatus::Pending,
        stage: MangaPagePipelineStage::Adaptation,
        attempt: 1,
        source_fingerprint: context.source_metadata.content_sha256.clone(),
        stage_fingerprints: BTreeMap::from([("content".into(), content_fingerprint)]),
        page_content: Some(slot.content.clone()),
        page_adaptation: None,
        storyboard: None,
        drawing_prompt: None,
        image: None,
        error: None,
    }
}

fn storyboard_fingerprint_input(
    context: &MangaRunContext<'_>,
    slot: &MangaPageSlotV1,
    adaptation: &MangaPageAdaptationV1,
    previous_content: Option<&MangaPageContentV1>,
    next_content: Option<&MangaPageContentV1>,
    extraction_summary: &Value,
) -> Value {
    json!({
        "page_content": slot.content,
        "page_adaptation": adaptation,
        "adaptation_beats": manga_page_adaptation_source(&context.adaptation, &slot.content),
        "previous_page_content": previous_content,
        "next_page_content": next_content,
        "characters": extraction_summary.get("characters"),
        "reference_assets": context.reference_assets,
    })
}

fn page_stage_fingerprint(
    context: &MangaRunContext<'_>,
    stage: MangaPagePipelineStage,
    input: Value,
) -> Result<String, AppError> {
    let fingerprint = serde_json::to_string(&json!({
        "pipeline_contract": "manga_page_pipeline_v2",
        "source": context.source_metadata,
        "prompt_fingerprint": context.prompt_fingerprint,
        "stage": stage.key(),
        "input": input,
    }))?;
    Ok(source_text_hash(&fingerprint))
}

pub(super) async fn process_manga_page(
    context: &MangaRunContext<'_>,
    persist: &MangaPipelinePersistInput<'_>,
    mut work: MangaPageWork,
    extraction_summary: &Value,
    progress: &MangaPagePipelineProgress,
    total_pages: usize,
) -> Result<MangaPagePromptRecord, AppError> {
    let result = process_manga_page_stages(
        context,
        persist,
        &mut work,
        extraction_summary,
        progress,
        total_pages,
    )
    .await;
    match result {
        Ok(()) => Ok(work.row),
        Err(error) => {
            let cancelled = matches!(&error, AppError::Cancelled);
            let active_stage = work.pipeline.stage;
            work.pipeline.status = if cancelled {
                MangaPagePipelineStatus::Cancelled
            } else {
                MangaPagePipelineStatus::Failed
            };
            work.pipeline.error = Some(page_pipeline_error(active_stage, &error, cancelled));
            match persist_page_state(context, persist, &mut work, Vec::new()).await {
                Ok(()) => Err(error),
                Err(persist_error) => Err(AppError::InvalidState {
                    current: format!("{error}；页面失败状态保存也失败：{persist_error}"),
                    operation: format!("保存漫画第 {} 页失败状态", work.slot.page_number),
                }),
            }
        }
    }
}

async fn process_manga_page_stages(
    context: &MangaRunContext<'_>,
    persist: &MangaPipelinePersistInput<'_>,
    work: &mut MangaPageWork,
    extraction_summary: &Value,
    progress: &MangaPagePipelineProgress,
    total_pages: usize,
) -> Result<(), AppError> {
    let page_number = work.slot.page_number;
    let content = work.slot.content.clone();
    context.check_cancelled()?;

    let adaptation_plan = manga_page_adaptation_source(&context.adaptation, &content);
    let adaptation_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Adaptation,
        json!({
            "page_content": &content,
            "adaptation_plan": &adaptation_plan,
            "previous_page_content": work.previous_content.as_ref(),
            "next_page_content": work.next_content.as_ref(),
        }),
    )?;
    if work.pipeline.page_adaptation.is_none()
        || work.pipeline.stage_fingerprints.get("adaptation") != Some(&adaptation_fingerprint)
    {
        work.pipeline.page_adaptation = None;
        work.pipeline.storyboard = None;
        work.pipeline.drawing_prompt = None;
        work.pipeline.image = None;
        work.pipeline.stage_fingerprints.remove("adaptation");
        work.pipeline.stage_fingerprints.remove("storyboard");
        work.pipeline.stage_fingerprints.remove("drawing_prompt");
        work.pipeline.stage_fingerprints.remove("image");
        work.pipeline.stage = MangaPagePipelineStage::Adaptation;
        work.pipeline.status = MangaPagePipelineStatus::Running;
        work.pipeline.error = None;
        persist_page_state(context, persist, work, Vec::new()).await?;
        context.check_cancelled()?;
        let user_content = manga_page_adaptation_user_message(
            page_number,
            total_pages,
            &content,
            adaptation_plan,
            work.previous_content.as_ref(),
            work.next_content.as_ref(),
        )?;
        let (trace, adaptation) = context
            .service
            .render_and_validate_manga_call(
                MangaRenderTraceInput {
                    prompt_renderer: context.prompt_renderer,
                    prompt_name: "manga_page_adaptation_plan",
                    project_id: context.project_id,
                    workflow_run_id: context.workflow_run_id,
                    call_key: format!(
                        "manga_pipeline:{}:page:{}:adaptation",
                        context.chapter_id, page_number
                    ),
                    call_order: manga_page_pipeline_call_order(work.page_index, 0)?,
                    restore_from_checkpoint: context.resume_from_checkpoint,
                    variables: &context.variables,
                    user_content,
                    temperature: 0.3,
                    cancel_token: context.cancel_token,
                    stream_id: format!("page_{page_number}_adaptation"),
                    stream_reporter: context.stream_reporter,
                },
                |value| parse_manga_page_adaptation(value, &content, "manga_page_adaptation_plan"),
            )
            .await?;
        work.pipeline.page_adaptation = Some(adaptation);
        work.pipeline
            .stage_fingerprints
            .insert("adaptation".into(), adaptation_fingerprint);
        work.pipeline.stage = MangaPagePipelineStage::Storyboard;
        work.pipeline.status = MangaPagePipelineStatus::Pending;
        persist_page_state(context, persist, work, vec![trace.clone()]).await?;
        progress
            .llm_usage(context, MangaPagePipelineStage::Adaptation, &trace)
            .await?;
    }
    progress
        .advance(
            context,
            page_number,
            total_pages,
            MangaPagePipelineStage::Adaptation,
            "页面改编计划已保存",
        )
        .await?;
    if stop_after_page_stage(context.stop_after, MangaPagePipelineStage::Adaptation) {
        return Ok(());
    }

    context.check_cancelled()?;
    let page_adaptation =
        work.pipeline
            .page_adaptation
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "page_adaptation_missing".into(),
                operation: format!("生成漫画第 {page_number} 页分镜"),
            })?;
    let storyboard_input = storyboard_fingerprint_input(
        context,
        &work.slot,
        &page_adaptation,
        work.previous_content.as_ref(),
        work.next_content.as_ref(),
        extraction_summary,
    );
    let storyboard_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Storyboard,
        storyboard_input,
    )?;
    if work.pipeline.storyboard.is_none()
        || work.pipeline.stage_fingerprints.get("storyboard") != Some(&storyboard_fingerprint)
    {
        work.pipeline.storyboard = None;
        work.pipeline.drawing_prompt = None;
        work.pipeline.image = None;
        work.pipeline.stage_fingerprints.remove("storyboard");
        work.pipeline.stage_fingerprints.remove("drawing_prompt");
        work.pipeline.stage_fingerprints.remove("image");
        work.pipeline.stage = MangaPagePipelineStage::Storyboard;
        work.pipeline.status = MangaPagePipelineStatus::Running;
        work.pipeline.error = None;
        persist_page_state(context, persist, work, Vec::new()).await?;
        context.check_cancelled()?;
        let adaptation_beats = manga_page_adaptation_source(&context.adaptation, &content);
        let user_content = manga_page_storyboard_user_message(
            page_number,
            total_pages,
            &content,
            &page_adaptation,
            work.previous_content.as_ref(),
            work.next_content.as_ref(),
            adaptation_beats,
            extraction_summary,
            &context.reference_assets,
        )?;
        let (trace, storyboard) = context
            .service
            .render_and_validate_manga_call(
                MangaRenderTraceInput {
                    prompt_renderer: context.prompt_renderer,
                    prompt_name: "manga_page_storyboard",
                    project_id: context.project_id,
                    workflow_run_id: context.workflow_run_id,
                    call_key: format!(
                        "manga_pipeline:{}:page:{}:storyboard",
                        context.chapter_id, page_number
                    ),
                    call_order: manga_page_pipeline_call_order(work.page_index, 1)?,
                    restore_from_checkpoint: context.resume_from_checkpoint,
                    variables: &context.variables,
                    user_content,
                    temperature: 0.3,
                    cancel_token: context.cancel_token,
                    stream_id: format!("page_{page_number}_storyboard"),
                    stream_reporter: context.stream_reporter,
                },
                |value| {
                    parse_manga_page_storyboard(
                        value,
                        page_number,
                        &content,
                        &page_adaptation,
                        "manga_page_storyboard",
                    )
                },
            )
            .await?;
        work.pipeline.storyboard = Some(storyboard);
        work.pipeline
            .stage_fingerprints
            .insert("storyboard".into(), storyboard_fingerprint);
        work.pipeline.stage = MangaPagePipelineStage::DrawingPrompt;
        work.pipeline.status = MangaPagePipelineStatus::Pending;
        persist_page_state(context, persist, work, vec![trace.clone()]).await?;
        progress
            .llm_usage(context, MangaPagePipelineStage::Storyboard, &trace)
            .await?;
    }
    progress
        .advance(
            context,
            page_number,
            total_pages,
            MangaPagePipelineStage::Storyboard,
            "页面分镜已保存",
        )
        .await?;
    if stop_after_page_stage(context.stop_after, MangaPagePipelineStage::Storyboard) {
        return Ok(());
    }

    context.check_cancelled()?;
    let storyboard = work
        .pipeline
        .storyboard
        .clone()
        .ok_or_else(|| AppError::InvalidState {
            current: "storyboard_missing".into(),
            operation: format!("生成漫画第 {page_number} 页绘图提示词"),
        })?;
    let drawing_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::DrawingPrompt,
        json!({
            "page_content": &content,
            "page_adaptation": work.pipeline.page_adaptation.as_ref(),
            "storyboard": &storyboard,
            "characters": extraction_summary.get("characters"),
            "reference_assets": context.reference_assets,
        }),
    )?;
    if work.pipeline.drawing_prompt.is_none()
        || work.pipeline.stage_fingerprints.get("drawing_prompt") != Some(&drawing_fingerprint)
    {
        work.pipeline.drawing_prompt = None;
        work.pipeline.image = None;
        work.pipeline.stage_fingerprints.remove("drawing_prompt");
        work.pipeline.stage_fingerprints.remove("image");
        work.pipeline.stage = MangaPagePipelineStage::DrawingPrompt;
        work.pipeline.status = MangaPagePipelineStatus::Running;
        work.pipeline.error = None;
        persist_page_state(context, persist, work, Vec::new()).await?;
        context.check_cancelled()?;
        let page_adaptation =
            work.pipeline
                .page_adaptation
                .as_ref()
                .ok_or_else(|| AppError::InvalidState {
                    current: "page_adaptation_missing".into(),
                    operation: format!("生成漫画第 {page_number} 页绘图提示词"),
                })?;
        let output = context
            .service
            .run_single_page_drawing_prompt(SinglePageDrawingPromptInput {
                chapter_id: context.chapter_id,
                project_id: context.project_id,
                prompt_renderer: context.prompt_renderer,
                vars: &context.variables,
                workflow_run_id: context.workflow_run_id,
                reference_assets: &context.reference_assets,
                page_content: &content,
                page_adaptation,
                page_plan: &storyboard,
                page_index: work.page_index,
                extraction_summary,
                restore_from_checkpoint: context.resume_from_checkpoint,
                cancel_token: context.cancel_token,
                stream_reporter: context.stream_reporter,
            })
            .await?;
        work.pipeline.drawing_prompt = Some(output.prompt);
        work.pipeline
            .stage_fingerprints
            .insert("drawing_prompt".into(), drawing_fingerprint);
        work.pipeline.image = None;
        work.pipeline.stage = MangaPagePipelineStage::Image;
        work.pipeline.status = MangaPagePipelineStatus::Pending;
        persist_page_state(context, persist, work, vec![output.trace.clone()]).await?;
        progress
            .llm_usage(
                context,
                MangaPagePipelineStage::DrawingPrompt,
                &output.trace,
            )
            .await?;
    }
    progress
        .advance(
            context,
            page_number,
            total_pages,
            MangaPagePipelineStage::DrawingPrompt,
            "整页提示词已保存",
        )
        .await?;
    if stop_after_page_stage(context.stop_after, MangaPagePipelineStage::DrawingPrompt) {
        return Ok(());
    }

    context.check_cancelled()?;
    let drawing_prompt =
        work.pipeline
            .drawing_prompt
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "drawing_prompt_missing".into(),
                operation: format!("生成漫画第 {page_number} 页图片"),
            })?;
    let reference_bindings = work
        .pipeline
        .storyboard
        .as_ref()
        .map(|page| {
            super::super::super::reference_assets::page_reference_bindings(
                page,
                &context.reference_assets_metadata,
            )
            .map_err(AppError::InvalidArgument)
        })
        .transpose()?
        .unwrap_or_default();
    let image_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Image,
        json!({
            "drawing_prompt": drawing_prompt,
            "reference_bindings": reference_bindings,
            "image_size": context.image_size,
        }),
    )?;
    let image_matches = work.pipeline.image.as_ref().is_some_and(|image| {
        Some(image.generated_image_id) == work.row.generated_image_id
            && work.row.status == "completed"
            && work.pipeline.stage_fingerprints.get("image") == Some(&image_fingerprint)
    });
    if !image_matches {
        work.pipeline.image = None;
        work.pipeline.stage_fingerprints.remove("image");
        work.pipeline.stage = MangaPagePipelineStage::Image;
        work.pipeline.status = MangaPagePipelineStatus::Running;
        work.pipeline.error = None;
        persist_page_state(context, persist, work, Vec::new()).await?;
        context.check_cancelled()?;
        let image_size = context
            .image_size
            .ok_or_else(|| AppError::InvalidArgument("自动生成漫画图片需要选择图片尺寸".into()))?;
        let generator = context
            .image_generator
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_page_image_generator_missing".into(),
                operation: "执行漫画自动成图".into(),
            })?;
        let cancel_token = context.cancel_token.cloned().unwrap_or_default();
        let reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(
                context,
                page_number,
                total_pages,
                &format!("正在成图 ({current}/{total})：{message}"),
            );
        };
        let image = generator
            .generate_page_image(
                context.user_id,
                GeneratePageImageInput {
                    project_id: context.project_id,
                    chapter_number: context.chapter_number,
                    page_number: i32::try_from(page_number).map_err(|_| {
                        AppError::InvalidArgument(format!("漫画页码超出 i32 范围：{page_number}"))
                    })?,
                    width: image_size.width,
                    height: image_size.height,
                },
                context.workflow_run_id,
                &reporter,
                cancel_token,
            )
            .await?;
        work.pipeline.image = Some(MangaPageImageArtifactV1 {
            workflow_run_id: context.workflow_run_id,
            generated_image_id: image.id,
            width: image.width,
            height: image.height,
        });
        work.pipeline
            .stage_fingerprints
            .insert("image".into(), image_fingerprint);
        work.pipeline.status = MangaPagePipelineStatus::Completed;
        work.pipeline.error = None;
        let existing_layout = manga_page_pipeline_layout(&work.row).ok();
        let draft = pipeline_page_draft(
            persist,
            page_number,
            work.pipeline.clone(),
            Vec::new(),
            existing_layout.as_ref(),
        )?;
        let layout = draft.layout.ok_or_else(|| AppError::InvalidState {
            current: "manga_page_layout_missing".into(),
            operation: format!("保存漫画第 {page_number} 页成图状态"),
        })?;
        let latest = context
            .service
            .manga_page_prompt_by_id(work.row.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("漫画第 {page_number} 页提示词已删除")))?;
        work.row = context
            .service
            .update_manga_page_prompt_layout(latest, layout)
            .await?;
    }
    progress
        .advance(
            context,
            page_number,
            total_pages,
            MangaPagePipelineStage::Image,
            "页面图片已生成",
        )
        .await?;
    work.pipeline.status = MangaPagePipelineStatus::Completed;
    Ok(())
}

async fn persist_page_state(
    context: &MangaRunContext<'_>,
    persist: &MangaPipelinePersistInput<'_>,
    work: &mut MangaPageWork,
    traces: Vec<MangaPromptGenerationTrace>,
) -> Result<(), AppError> {
    let existing_layout = manga_page_pipeline_layout(&work.row).ok();
    let draft = pipeline_page_draft(
        persist,
        work.slot.page_number,
        work.pipeline.clone(),
        traces,
        existing_layout.as_ref(),
    )?;
    work.row = context
        .service
        .upsert_generated_page_prompt(context.project_id, context.chapter_id, draft)
        .await?;
    Ok(())
}

fn stop_after_page_stage(
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    stage: MangaPagePipelineStage,
) -> bool {
    matches!(
        (stop_after, stage),
        (
            Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation),
            MangaPagePipelineStage::Adaptation,
        ) | (
            Some(
                a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard
                    | a4n_api_contract::manga::MangaPipelineStopAfter::PagePlanning,
            ),
            MangaPagePipelineStage::Storyboard,
        ) | (
            Some(a4n_api_contract::manga::MangaPipelineStopAfter::DrawingPrompt),
            MangaPagePipelineStage::DrawingPrompt,
        )
    )
}

fn page_pipeline_error(
    stage: MangaPagePipelineStage,
    error: &AppError,
    cancelled: bool,
) -> MangaPagePipelineErrorV1 {
    let retryable = matches!(
        error,
        AppError::LlmError(_) | AppError::Timeout(_) | AppError::Internal(_) | AppError::Io(_)
    );
    MangaPagePipelineErrorV1 {
        stage,
        code: if cancelled {
            "cancelled"
        } else {
            "page_stage_failed"
        }
        .to_string(),
        message: error.to_string(),
        retryable: !cancelled && retryable,
    }
}
