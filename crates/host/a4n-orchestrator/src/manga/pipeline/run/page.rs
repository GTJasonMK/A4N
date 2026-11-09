use std::{
    collections::{BTreeMap, HashSet},
    sync::atomic::{AtomicU32, Ordering},
};

use a4n_api_contract::image_generation::GeneratePageImageInput;
use a4n_db::manga::MANGA_PAGE_PROMPT_TYPE;
use a4n_domain::{
    manga::{
        MANGA_PAGE_PIPELINE_STATE_V1, MangaPageAdaptationV1, MangaPageContentV1,
        MangaPageGenerationLayoutV4, MangaPageImageArtifactV1, MangaPagePipelineErrorV1,
        MangaPagePipelineStage, MangaPagePipelineStateV1, MangaPagePipelineStatus,
        MangaPagePromptRecord, MangaPageSlotV1, MangaPageSlotsV1, MangaPipelineArtifactDraft,
        MangaPipelineArtifactRecord, MangaPipelineArtifactStage, MangaReferenceAssetKind,
        MangaReferenceBindingSummary,
    },
    media::GeneratedImageRecord,
    source_text_hash,
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::manga::{is_manga_visual_design_range, layout_rules::manga_page_pipeline_layout};

use super::super::super::adaptation_projection::manga_page_beats_source;
use super::super::super::reference_assets::{
    manga_page_reference_discovery_candidates, parse_manga_page_reference_discovery,
};
use super::super::{
    json::{parse_manga_page_adaptation, parse_manga_page_storyboard},
    persist::{MangaPipelinePersistInput, pipeline_page_draft},
    prompt::{
        MangaPromptGenerationTrace, MangaRenderTraceInput, manga_page_adaptation_user_message,
        manga_page_reference_discovery_user_message, manga_page_source_input,
        manga_page_storyboard_user_message,
    },
    state::{manga_page_pipeline_call_order, manga_page_planning_total_steps},
};
use super::{
    contracts::{MangaRunContext, should_generate_manga_stage},
    drawing_prompt::SinglePageDrawingPromptInput,
};

pub(super) const MANGA_PAGE_PIPELINE_CONCURRENCY: usize = 2;
const PAGE_PIPELINE_STAGES: [MangaPagePipelineStage; 5] = [
    MangaPagePipelineStage::Adaptation,
    MangaPagePipelineStage::Storyboard,
    MangaPagePipelineStage::ReferenceDiscovery,
    MangaPagePipelineStage::DrawingPrompt,
    MangaPagePipelineStage::Image,
];

pub(super) struct MangaPageWork {
    pub(super) slot: MangaPageSlotV1,
    pub(super) page_index: usize,
    pub(super) previous_content: Option<MangaPageContentV1>,
    pub(super) next_content: Option<MangaPageContentV1>,
    pub(super) pipeline: MangaPagePipelineStateV1,
    pub(super) stale_stages: HashSet<MangaPipelineArtifactStage>,
    pub(super) changed_stage: Option<MangaPipelineArtifactStage>,
    pub(super) row: MangaPagePromptRecord,
}

pub(super) struct MangaPagePipelineProgress {
    completed: AtomicU32,
    total: i32,
    base: i32,
    lock: Mutex<()>,
}

impl MangaPagePipelineProgress {
    pub(super) fn new(total: i32, base: i32) -> Self {
        Self {
            completed: AtomicU32::new(0),
            total,
            base,
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
            self.base
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
            stage.key(),
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
            Some(u64::from(page_number.saturating_sub(1))),
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
        let current = self
            .base
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
        let current = self.base + completed;
        let label = format!("第 {page_number}/{total_pages} 页：{message}");
        context.publish_progress(
            "image_progress",
            ProgressPhaseKind::AssetGeneration,
            current,
            self.total,
            &label,
            "页",
            Some(format!("page-{page_number:04}")),
            Some(u64::from(page_number.saturating_sub(1))),
            u64::try_from(total_pages).ok(),
        );
    }
}

pub(super) fn manga_page_pipeline_total_steps(
    page_count: usize,
    total_page_count: usize,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Result<i32, AppError> {
    let stages_per_page = match stop_after {
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots) => 0,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::PageAdaptation) => 1,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard) => 2,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::ReferenceDiscovery) => 3,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::DrawingPrompt) => 4,
        Some(a4n_api_contract::manga::MangaPipelineStopAfter::Image) | None => {
            i32::try_from(PAGE_PIPELINE_STAGES.len())
                .map_err(|_| AppError::InvalidArgument("漫画页面阶段数超出 i32 范围".into()))?
        }
    };
    let page_steps = i32::try_from(page_count)
        .ok()
        .and_then(|pages| pages.checked_mul(stages_per_page))
        .ok_or_else(|| AppError::InvalidArgument("漫画页面进度总数超出 i32 范围".into()))?;
    manga_page_planning_total_steps(total_page_count)?
        .checked_add(page_steps)
        .ok_or_else(|| AppError::InvalidArgument("漫画页面进度总数超出 i32 范围".into()))
}

pub(super) fn prepare_manga_page_work(
    context: &MangaRunContext<'_>,
    slots: &MangaPageSlotsV1,
    page_index: usize,
    extraction_summary: &Value,
    artifacts: &[MangaPipelineArtifactRecord],
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
    let content_fingerprint = match artifacts.iter().find(|artifact| {
        artifact.stage == MangaPipelineArtifactStage::PageContent
            && artifact.page_number == Some(slot.page_number)
            && !artifact.stale
    }) {
        Some(artifact) => artifact.input_fingerprint.clone(),
        None => page_stage_fingerprint(
            context,
            MangaPagePipelineStage::Content,
            json!({"page_content": slot.content}),
        )?,
    };
    let adaptation_plan = manga_page_beats_source(&context.adaptation, &slot.content.beat_refs);
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

    // A page row is the compatibility projection for all page stages. Reuse it
    // by page identity even when its inputs changed; the fingerprints below
    // mark old outputs stale without discarding them.
    let reusable_layout = existing_row
        .as_ref()
        .map(manga_page_pipeline_layout)
        .transpose()
        .map_err(AppError::InvalidArgument)?;
    let reference_bindings = reusable_layout
        .as_ref()
        .map_or_else(Vec::new, |layout| layout.reference_bindings.clone());
    let previous_attempt = reusable_layout
        .as_ref()
        .map(|layout| layout.pipeline.attempt);
    let mut pipeline = reusable_layout.map_or_else(
        || new_page_pipeline(context, slot, content_fingerprint.clone()),
        |layout| layout.pipeline,
    );
    pipeline.source_fingerprint = context.source_metadata.content_sha256.clone();
    pipeline.page_content = Some(slot.content.clone());
    let mut stale_stages = HashSet::new();
    for artifact in artifacts
        .iter()
        .filter(|artifact| artifact.page_number == Some(slot.page_number))
    {
        match artifact.stage {
            MangaPipelineArtifactStage::PageContent => continue,
            MangaPipelineArtifactStage::PageAdaptation => {
                pipeline.page_adaptation =
                    Some(parse_manga_stage_payload(artifact, "page_adaptation")?);
            }
            MangaPipelineArtifactStage::Storyboard => {
                pipeline.storyboard = Some(parse_manga_stage_payload(artifact, "storyboard")?);
            }
            MangaPipelineArtifactStage::ReferenceDiscovery => {
                pipeline.reference_discovery =
                    Some(parse_manga_stage_payload(artifact, "reference_discovery")?);
            }
            MangaPipelineArtifactStage::DrawingPrompt => {
                pipeline.drawing_prompt =
                    Some(parse_manga_stage_payload(artifact, "drawing_prompt")?);
            }
            MangaPipelineArtifactStage::Image => {
                pipeline.image = Some(parse_manga_stage_payload(artifact, "image")?);
            }
            _ => continue,
        }
        pipeline.stage_fingerprints.insert(
            artifact.stage.fingerprint_key().into(),
            artifact.input_fingerprint.clone(),
        );
        if artifact.stale {
            stale_stages.insert(artifact.stage);
        }
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
                )?,
            )
        })
        .transpose()?;
    if let (Some(row), Some(drawing_prompt)) =
        (existing_row.as_ref(), pipeline.drawing_prompt.as_mut())
        && !row.image_prompt.trim().is_empty()
        && row.image_prompt != drawing_prompt.image_prompt
    {
        drawing_prompt.image_prompt = row.image_prompt.clone();
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
                "reference_discovery": &pipeline.reference_discovery,
                "reference_bindings": &reference_bindings,
                "user_reference_image_descriptions": user_reference_image_descriptions(
                    &reference_bindings,
                    &context.available_reference_images,
                ),
            }),
        )
    });
    let drawing_fingerprint = drawing_fingerprint.transpose()?;
    let image_matches = if let Some(drawing_prompt) = pipeline.drawing_prompt.as_ref() {
        let system_reference_images = pipeline
            .reference_discovery
            .as_ref()
            .map_or(&[][..], |discovery| discovery.references.as_slice());
        let image_fingerprint = page_stage_fingerprint(
            context,
            MangaPagePipelineStage::Image,
            json!({
                "drawing_prompt": drawing_prompt,
                "reference_bindings": &reference_bindings,
                "system_reference_images": system_reference_images,
                "image_size": context.image_size,
            }),
        )?;
        !stale_stages.contains(&MangaPipelineArtifactStage::Image)
            && pipeline.image.as_ref().is_some_and(|image| {
                Some(image.generated_image_id)
                    == existing_row.as_ref().and_then(|row| row.generated_image_id)
                    && existing_row
                        .as_ref()
                        .is_some_and(|row| row.status == "completed")
                    && pipeline.stage_fingerprints.get("image") == Some(&image_fingerprint)
            })
    } else {
        false
    };

    let adaptation_is_current = pipeline.page_adaptation.is_some()
        && !stale_stages.contains(&MangaPipelineArtifactStage::PageAdaptation)
        && pipeline.stage_fingerprints.get("adaptation") == Some(&adaptation_fingerprint);
    let storyboard_is_current = pipeline.storyboard.is_some()
        && !stale_stages.contains(&MangaPipelineArtifactStage::Storyboard)
        && pipeline.stage_fingerprints.get("storyboard") == storyboard_fingerprint.as_ref();
    let drawing_prompt_is_current = pipeline.drawing_prompt.is_some()
        && !stale_stages.contains(&MangaPipelineArtifactStage::DrawingPrompt)
        && pipeline.stage_fingerprints.get("drawing_prompt") == drawing_fingerprint.as_ref();
    let next_stage = if !adaptation_is_current {
        MangaPagePipelineStage::Adaptation
    } else if !storyboard_is_current {
        MangaPagePipelineStage::Storyboard
    } else if !drawing_prompt_is_current {
        MangaPagePipelineStage::DrawingPrompt
    } else {
        MangaPagePipelineStage::Image
    };
    let is_existing_complete = image_matches
        && existing_row
            .as_ref()
            .is_some_and(|row| row.status == "completed");
    pipeline.attempt = if is_existing_complete {
        previous_attempt.unwrap_or(1)
    } else {
        previous_attempt
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
        stale_stages,
        changed_stage: None,
        row,
    })
}

fn parse_manga_stage_payload<T: serde::de::DeserializeOwned>(
    artifact: &MangaPipelineArtifactRecord,
    field: &str,
) -> Result<T, AppError> {
    serde_json::from_value(artifact.payload.clone()).map_err(|error| {
        AppError::InvalidArgument(format!(
            "漫画阶段 {} 的 {field} 产物无法读取：{error}",
            artifact.stage.key()
        ))
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
        reference_discovery: None,
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
) -> Result<Value, AppError> {
    Ok(json!({
        "page_content": slot.content,
        "page_adaptation": adaptation,
        "adaptation_beats": manga_page_beats_source(&context.adaptation, &slot.content.beat_refs),
        "previous_page_content": previous_content,
        "next_page_content": next_content,
        "characters": extraction_summary.get("characters"),
        "reference_assets": context.reference_assets,
        "source_context": manga_page_source_input(
            &slot.content.source_span_ids, extraction_summary, &context.source_spans,
            &context.chapter_content, &context.source_markers,
        )?,
    }))
}

fn user_reference_image_descriptions(
    bindings: &[MangaReferenceBindingSummary],
    available_images: &[GeneratedImageRecord],
) -> Value {
    let user_image_ids = bindings
        .iter()
        .flat_map(|binding| {
            binding.user_added_assets.iter().chain(
                binding
                    .bound_asset
                    .iter()
                    .filter(|asset| asset.asset_kind == MangaReferenceAssetKind::GeneratedImage),
            )
        })
        .filter(|asset| asset.asset_kind == MangaReferenceAssetKind::GeneratedImage)
        .map(|asset| asset.asset_id)
        .collect::<HashSet<_>>();
    Value::Array(
        available_images
            .iter()
            .filter(|image| user_image_ids.contains(&image.id))
            .map(|image| {
                let description = image.prompt.trim();
                json!({
                    "asset_id": image.id,
                    "description": if description.is_empty() {
                        "用户添加的图片参考"
                    } else {
                        description
                    },
                })
            })
            .collect(),
    )
}

fn reference_discovery_is_usable_for_stage(
    run_stage: Option<MangaPipelineArtifactStage>,
    artifact_is_current: bool,
    fingerprint_matches: bool,
) -> bool {
    artifact_is_current
        && (matches!(
            run_stage,
            Some(MangaPipelineArtifactStage::DrawingPrompt | MangaPipelineArtifactStage::Image)
        ) || fingerprint_matches)
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
    let run_stage = context.run_stage;
    let visual_design_range = is_manga_visual_design_range(run_stage, context.stop_after);
    context.check_cancelled()?;
    if visual_design_range
        && work.pipeline.reference_discovery.is_some()
        && (!context.resume_from_checkpoint
            || work
                .stale_stages
                .contains(&MangaPipelineArtifactStage::ReferenceDiscovery)
            || work.pipeline.storyboard.is_none())
    {
        return Err(super::contracts::manga_stage_output_exists_error(
            MangaPipelineArtifactStage::ReferenceDiscovery,
            Some(work.slot.page_number),
        ));
    }

    let adaptation_plan = manga_page_beats_source(&context.adaptation, &content.beat_refs);
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
    let adaptation_is_current = work.pipeline.page_adaptation.is_some()
        && !work
            .stale_stages
            .contains(&MangaPipelineArtifactStage::PageAdaptation)
        && work.pipeline.stage_fingerprints.get("adaptation") == Some(&adaptation_fingerprint);
    let adaptation_requested = run_stage == Some(MangaPipelineArtifactStage::PageAdaptation);
    let generate_adaptation = should_generate_manga_stage(
        MangaPipelineArtifactStage::PageAdaptation,
        page_number,
        (run_stage.is_none() || adaptation_requested)
            && (adaptation_requested || !adaptation_is_current),
        adaptation_requested,
        work.pipeline.page_adaptation.is_some(),
        adaptation_is_current,
        context.resume_from_checkpoint,
    )?;
    if run_stage.is_some()
        && run_stage != Some(MangaPipelineArtifactStage::PageAdaptation)
        && !adaptation_is_current
    {
        return Err(AppError::InvalidState {
            current: "manga_stage_artifact_stale:adaptation".into(),
            operation: format!("生成漫画第 {page_number} 页下游阶段前，请先重跑页面改编"),
        });
    }
    if generate_adaptation {
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
            &context.source_markers,
        )?;
        let (trace, adaptation) = context
            .service
            .render_and_validate_manga_call(
                MangaRenderTraceInput {
                    prompt_renderer: context.prompt_renderer,
                    prompt_name: "manga_page_adaptation_plan",
                    project_id: context.project_id,
                    workflow_run_id: context.workflow_run_id,
                    call_key: a4n_domain::manga::MangaModelCall::PageAdaptation(page_number)
                        .call_key(context.chapter_id),
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
        work.stale_stages
            .remove(&MangaPipelineArtifactStage::PageAdaptation);
        work.changed_stage = Some(MangaPipelineArtifactStage::PageAdaptation);
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
    if run_stage.is_none()
        || run_stage == Some(MangaPipelineArtifactStage::PageAdaptation)
        || visual_design_range
    {
        progress
            .advance(
                context,
                page_number,
                total_pages,
                MangaPagePipelineStage::Adaptation,
                "页面改编计划已保存",
            )
            .await?;
    }
    if run_stage == Some(MangaPipelineArtifactStage::PageAdaptation)
        || (run_stage.is_none()
            && stop_after_page_stage(context.stop_after, MangaPagePipelineStage::Adaptation))
    {
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
    )?;
    let storyboard_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Storyboard,
        storyboard_input,
    )?;
    let storyboard_is_current = work.pipeline.storyboard.is_some()
        && !work
            .stale_stages
            .contains(&MangaPipelineArtifactStage::Storyboard)
        && work.pipeline.stage_fingerprints.get("storyboard") == Some(&storyboard_fingerprint);
    let storyboard_requested = run_stage == Some(MangaPipelineArtifactStage::Storyboard);
    let generate_storyboard = should_generate_manga_stage(
        MangaPipelineArtifactStage::Storyboard,
        page_number,
        (run_stage.is_none() || storyboard_requested)
            && (storyboard_requested || !storyboard_is_current),
        storyboard_requested,
        work.pipeline.storyboard.is_some(),
        storyboard_is_current,
        context.resume_from_checkpoint,
    )?;
    if run_stage.is_some()
        && run_stage != Some(MangaPipelineArtifactStage::Storyboard)
        && !storyboard_is_current
    {
        return Err(AppError::InvalidState {
            current: "manga_stage_artifact_stale:storyboard".into(),
            operation: format!("生成漫画第 {page_number} 页下游阶段前，请先重跑分镜"),
        });
    }
    if generate_storyboard {
        work.pipeline.stage = MangaPagePipelineStage::Storyboard;
        work.pipeline.status = MangaPagePipelineStatus::Running;
        work.pipeline.error = None;
        persist_page_state(context, persist, work, Vec::new()).await?;
        context.check_cancelled()?;
        let adaptation_beats = manga_page_beats_source(&context.adaptation, &content.beat_refs);
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
            &context.source_spans,
            &context.chapter_content,
            &context.source_markers,
        )?;
        let (trace, storyboard) = context
            .service
            .render_and_validate_manga_call(
                MangaRenderTraceInput {
                    prompt_renderer: context.prompt_renderer,
                    prompt_name: "manga_page_storyboard",
                    project_id: context.project_id,
                    workflow_run_id: context.workflow_run_id,
                    call_key: a4n_domain::manga::MangaModelCall::Storyboard(page_number)
                        .call_key(context.chapter_id),
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
        work.stale_stages
            .remove(&MangaPipelineArtifactStage::Storyboard);
        work.changed_stage = Some(MangaPipelineArtifactStage::Storyboard);
        work.pipeline
            .stage_fingerprints
            .insert("storyboard".into(), storyboard_fingerprint);
        work.pipeline.stage = MangaPagePipelineStage::ReferenceDiscovery;
        work.pipeline.status = MangaPagePipelineStatus::Pending;
        persist_page_state(context, persist, work, vec![trace.clone()]).await?;
        progress
            .llm_usage(context, MangaPagePipelineStage::Storyboard, &trace)
            .await?;
    }
    if run_stage.is_none() || run_stage == Some(MangaPipelineArtifactStage::Storyboard) {
        progress
            .advance(
                context,
                page_number,
                total_pages,
                MangaPagePipelineStage::Storyboard,
                "页面分镜已保存",
            )
            .await?;
    }
    if (run_stage == Some(MangaPipelineArtifactStage::Storyboard) && !visual_design_range)
        || (run_stage.is_none()
            && stop_after_page_stage(context.stop_after, MangaPagePipelineStage::Storyboard))
    {
        return Ok(());
    }

    context.check_cancelled()?;
    let storyboard = work
        .pipeline
        .storyboard
        .clone()
        .ok_or_else(|| AppError::InvalidState {
            current: "storyboard_missing".into(),
            operation: format!("为漫画第 {page_number} 页发现参考素材"),
        })?;
    let current_layout =
        manga_page_pipeline_layout(&work.row).map_err(AppError::InvalidArgument)?;
    let (candidate_assets, candidates) = manga_page_reference_discovery_candidates(
        &context.available_reference_images,
        work.row.generated_image_id,
        &current_layout.reference_bindings,
    );
    let reference_discovery_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::ReferenceDiscovery,
        json!({
            "page_content": &content,
            "page_adaptation": work.pipeline.page_adaptation.as_ref(),
            "storyboard": &storyboard,
            "characters": extraction_summary.get("characters"),
            "available_assets": &candidate_assets,
            "reference_bindings": &current_layout.reference_bindings,
        }),
    )?;
    let reference_discovery_artifact_is_current = work.pipeline.reference_discovery.is_some()
        && !work
            .stale_stages
            .contains(&MangaPipelineArtifactStage::ReferenceDiscovery);
    let reference_discovery_fingerprint_matches =
        work.pipeline.stage_fingerprints.get("reference_discovery")
            == Some(&reference_discovery_fingerprint);
    let reference_discovery_is_current =
        reference_discovery_artifact_is_current && reference_discovery_fingerprint_matches;
    let reference_discovery_is_usable = !visual_design_range
        && reference_discovery_is_usable_for_stage(
            run_stage,
            reference_discovery_artifact_is_current,
            reference_discovery_fingerprint_matches,
        );
    if run_stage.is_some()
        && run_stage != Some(MangaPipelineArtifactStage::ReferenceDiscovery)
        && !visual_design_range
        && !reference_discovery_is_usable
    {
        return Err(AppError::InvalidState {
            current: "manga_stage_artifact_stale:reference_discovery".into(),
            operation: format!("生成漫画第 {page_number} 页下游阶段前，请先重跑参考图发现"),
        });
    }
    let reference_discovery_requested =
        run_stage == Some(MangaPipelineArtifactStage::ReferenceDiscovery) || visual_design_range;
    let generate_reference_discovery = should_generate_manga_stage(
        MangaPipelineArtifactStage::ReferenceDiscovery,
        page_number,
        reference_discovery_requested || (run_stage.is_none() && !reference_discovery_is_current),
        reference_discovery_requested,
        work.pipeline.reference_discovery.is_some(),
        reference_discovery_is_current,
        context.resume_from_checkpoint,
    )?;
    if generate_reference_discovery {
        work.pipeline.stage = MangaPagePipelineStage::ReferenceDiscovery;
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
                    operation: format!("为漫画第 {page_number} 页发现参考素材"),
                })?;
        let user_content = manga_page_reference_discovery_user_message(
            page_number,
            &storyboard,
            &content,
            page_adaptation,
            extraction_summary,
            candidate_assets,
            &context.source_markers,
        )?;
        let (trace, discovery) = context
            .service
            .render_and_validate_manga_call(
                MangaRenderTraceInput {
                    prompt_renderer: context.prompt_renderer,
                    prompt_name: "manga_page_reference_discovery",
                    project_id: context.project_id,
                    workflow_run_id: context.workflow_run_id,
                    call_key: a4n_domain::manga::MangaModelCall::ReferenceDiscovery(page_number)
                        .call_key(context.chapter_id),
                    call_order: manga_page_pipeline_call_order(work.page_index, 2)?,
                    restore_from_checkpoint: context.resume_from_checkpoint,
                    variables: &context.variables,
                    user_content,
                    temperature: 0.2,
                    cancel_token: context.cancel_token,
                    stream_id: format!("page_{page_number}_reference_discovery"),
                    stream_reporter: context.stream_reporter,
                },
                |value| parse_manga_page_reference_discovery(value, page_number, &candidates),
            )
            .await?;
        work.pipeline.reference_discovery = Some(discovery);
        work.stale_stages
            .remove(&MangaPipelineArtifactStage::ReferenceDiscovery);
        work.changed_stage = Some(MangaPipelineArtifactStage::ReferenceDiscovery);
        work.pipeline.stage_fingerprints.insert(
            "reference_discovery".into(),
            reference_discovery_fingerprint,
        );
        work.pipeline.stage = MangaPagePipelineStage::DrawingPrompt;
        work.pipeline.status = MangaPagePipelineStatus::Pending;
        persist_page_state(context, persist, work, vec![trace.clone()]).await?;
        progress
            .llm_usage(context, MangaPagePipelineStage::ReferenceDiscovery, &trace)
            .await?;
    }
    let reference_count = work
        .pipeline
        .reference_discovery
        .as_ref()
        .map_or(0, |discovery| discovery.references.len());
    if run_stage.is_none()
        || run_stage == Some(MangaPipelineArtifactStage::ReferenceDiscovery)
        || visual_design_range
    {
        progress
            .advance(
                context,
                page_number,
                total_pages,
                MangaPagePipelineStage::ReferenceDiscovery,
                &format!("已准备 {reference_count} 张参考图"),
            )
            .await?;
    }
    if visual_design_range
        || run_stage == Some(MangaPipelineArtifactStage::ReferenceDiscovery)
        || (run_stage.is_none()
            && stop_after_page_stage(
                context.stop_after,
                MangaPagePipelineStage::ReferenceDiscovery,
            ))
    {
        return Ok(());
    }

    context.check_cancelled()?;
    let reference_discovery =
        work.pipeline
            .reference_discovery
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "reference_discovery_missing".into(),
                operation: format!("生成漫画第 {page_number} 页绘图提示词"),
            })?;
    let current_layout =
        manga_page_pipeline_layout(&work.row).map_err(AppError::InvalidArgument)?;
    let drawing_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::DrawingPrompt,
        json!({
            "page_content": &content,
            "page_adaptation": work.pipeline.page_adaptation.as_ref(),
            "storyboard": &storyboard,
            "characters": extraction_summary.get("characters"),
            "reference_assets": context.reference_assets,
            "reference_discovery": reference_discovery,
            "reference_bindings": &current_layout.reference_bindings,
            "user_reference_image_descriptions": user_reference_image_descriptions(
                &current_layout.reference_bindings,
                &context.available_reference_images,
            ),
        }),
    )?;
    let drawing_prompt_is_current = work.pipeline.drawing_prompt.is_some()
        && !work
            .stale_stages
            .contains(&MangaPipelineArtifactStage::DrawingPrompt)
        && work.pipeline.stage_fingerprints.get("drawing_prompt") == Some(&drawing_fingerprint);
    let drawing_prompt_requested = run_stage == Some(MangaPipelineArtifactStage::DrawingPrompt);
    let generate_drawing_prompt = should_generate_manga_stage(
        MangaPipelineArtifactStage::DrawingPrompt,
        page_number,
        (run_stage.is_none() || drawing_prompt_requested)
            && (drawing_prompt_requested || !drawing_prompt_is_current),
        drawing_prompt_requested,
        work.pipeline.drawing_prompt.is_some(),
        drawing_prompt_is_current,
        context.resume_from_checkpoint,
    )?;
    if run_stage == Some(MangaPipelineArtifactStage::Image) && !drawing_prompt_is_current {
        return Err(AppError::InvalidState {
            current: "manga_stage_artifact_stale:drawing_prompt".into(),
            operation: format!("生成漫画第 {page_number} 页图片前，请先重跑绘图提示词"),
        });
    }
    if generate_drawing_prompt {
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
                reference_bindings: &current_layout.reference_bindings,
                reference_requirements: &reference_discovery.requirements,
                system_reference_images: &reference_discovery.references,
                available_reference_images: &context.available_reference_images,
                page_content: &content,
                page_adaptation,
                page_plan: &storyboard,
                page_index: work.page_index,
                extraction_summary,
                source_markers: &context.source_markers,
                restore_from_checkpoint: context.resume_from_checkpoint,
                cancel_token: context.cancel_token,
                stream_reporter: context.stream_reporter,
            })
            .await?;
        work.pipeline.drawing_prompt = Some(output.prompt);
        work.stale_stages
            .remove(&MangaPipelineArtifactStage::DrawingPrompt);
        work.changed_stage = Some(MangaPipelineArtifactStage::DrawingPrompt);
        work.pipeline
            .stage_fingerprints
            .insert("drawing_prompt".into(), drawing_fingerprint);
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
    if run_stage.is_none() || run_stage == Some(MangaPipelineArtifactStage::DrawingPrompt) {
        progress
            .advance(
                context,
                page_number,
                total_pages,
                MangaPagePipelineStage::DrawingPrompt,
                "整页提示词已保存",
            )
            .await?;
    }
    if run_stage == Some(MangaPipelineArtifactStage::DrawingPrompt)
        || (run_stage.is_none()
            && stop_after_page_stage(context.stop_after, MangaPagePipelineStage::DrawingPrompt))
    {
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
    let current_layout =
        manga_page_pipeline_layout(&work.row).map_err(AppError::InvalidArgument)?;
    let reference_bindings = &current_layout.reference_bindings;
    let system_reference_images = work
        .pipeline
        .reference_discovery
        .as_ref()
        .map_or(&[][..], |discovery| discovery.references.as_slice());
    let image_fingerprint = page_stage_fingerprint(
        context,
        MangaPagePipelineStage::Image,
        json!({
            "drawing_prompt": drawing_prompt,
            "reference_bindings": reference_bindings,
            "system_reference_images": system_reference_images,
            "image_size": context.image_size,
        }),
    )?;
    let image_matches = work.pipeline.image.as_ref().is_some_and(|image| {
        !work
            .stale_stages
            .contains(&MangaPipelineArtifactStage::Image)
            && Some(image.generated_image_id) == work.row.generated_image_id
            && work.row.status == "completed"
            && work.pipeline.stage_fingerprints.get("image") == Some(&image_fingerprint)
    });
    let image_requested = run_stage == Some(MangaPipelineArtifactStage::Image);
    let generate_image = should_generate_manga_stage(
        MangaPipelineArtifactStage::Image,
        page_number,
        !image_matches || image_requested,
        image_requested,
        work.pipeline.image.is_some() || work.row.generated_image_id.is_some(),
        image_matches,
        context.resume_from_checkpoint,
    )?;
    if generate_image {
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
            workflow_run_id: Some(context.workflow_run_id),
            generated_image_id: image.id,
            width: image.width,
            height: image.height,
        });
        work.stale_stages.remove(&MangaPipelineArtifactStage::Image);
        work.pipeline
            .stage_fingerprints
            .insert("image".into(), image_fingerprint);
        work.pipeline.status = MangaPagePipelineStatus::Completed;
        work.pipeline.error = None;
        let existing_layout =
            manga_page_pipeline_layout(&work.row).map_err(AppError::InvalidArgument)?;
        let draft = pipeline_page_draft(
            persist,
            page_number,
            work.pipeline.clone(),
            Vec::new(),
            Some(&existing_layout),
        )?;
        let layout = draft.layout.ok_or_else(|| AppError::InvalidState {
            current: "manga_page_layout_missing".into(),
            operation: format!("保存漫画第 {page_number} 页成图状态"),
        })?;
        let old_layout = work.row.layout.clone();
        let new_layout = layout.clone();
        let latest = context
            .service
            .manga_page_prompt_by_id(work.row.id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("漫画第 {page_number} 页提示词已删除")))?;
        work.row = context
            .service
            .update_manga_page_prompt_layout(latest, layout)
            .await?;
        persist_page_pipeline_artifact_delta(
            context,
            page_number,
            old_layout.as_ref(),
            Some(&new_layout),
            Some(MangaPipelineArtifactStage::Image),
        )
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
    let old_layout = work.row.layout.clone();
    let existing_layout = if work.row.id == 0 {
        None
    } else {
        Some(manga_page_pipeline_layout(&work.row).map_err(AppError::InvalidArgument)?)
    };
    let draft = pipeline_page_draft(
        persist,
        work.slot.page_number,
        work.pipeline.clone(),
        traces,
        existing_layout.as_ref(),
    )?;
    let new_layout = draft.layout.clone();
    work.row = context
        .service
        .upsert_generated_page_prompt(context.project_id, context.chapter_id, draft)
        .await?;
    persist_page_pipeline_artifact_delta(
        context,
        work.slot.page_number,
        old_layout.as_ref(),
        new_layout.as_ref(),
        work.changed_stage,
    )
    .await?;
    work.changed_stage = None;
    Ok(())
}

pub(super) async fn persist_page_pipeline_artifact_delta(
    context: &MangaRunContext<'_>,
    page_number: u32,
    old_layout: Option<&Value>,
    new_layout: Option<&Value>,
    force_stage: Option<MangaPipelineArtifactStage>,
) -> Result<(), AppError> {
    let legacy_v3 = old_layout
        .and_then(|layout| layout.get("schema_version"))
        .and_then(Value::as_str)
        == Some("manga_page_generation_v3");
    let old = match old_layout {
        Some(layout) if legacy_v3 => Some(
            a4n_manga_core::manga_page_pipeline_layout_from_value(page_number, layout)
                .map_err(AppError::InvalidArgument)?,
        ),
        Some(layout) => Some(serde_json::from_value::<MangaPageGenerationLayoutV4>(
            layout.clone(),
        )?),
        None => None,
    };
    let new = new_layout
        .cloned()
        .map(serde_json::from_value::<MangaPageGenerationLayoutV4>)
        .transpose()?;
    let Some(new) = new else {
        return Ok(());
    };
    let page = Some(page_number);
    let old_pipeline = old.as_ref().map(|layout| &layout.pipeline);
    let new_pipeline = &new.pipeline;
    let old_storyboard_fingerprint = if legacy_v3
        && old_pipeline.and_then(|pipeline| pipeline.storyboard.as_ref())
            == new_pipeline.storyboard.as_ref()
    {
        new_pipeline.stage_fingerprints.get("storyboard")
    } else {
        old_pipeline.and_then(|pipeline| pipeline.stage_fingerprints.get("storyboard"))
    };
    let old_drawing_prompt_fingerprint = if legacy_v3
        && old_pipeline.and_then(|pipeline| pipeline.drawing_prompt.as_ref())
            == new_pipeline.drawing_prompt.as_ref()
    {
        new_pipeline.stage_fingerprints.get("drawing_prompt")
    } else {
        old_pipeline.and_then(|pipeline| pipeline.stage_fingerprints.get("drawing_prompt"))
    };
    persist_page_stage_artifact_if_changed(
        context,
        page,
        MangaPipelineArtifactStage::PageContent,
        old_pipeline.and_then(|pipeline| pipeline.page_content.as_ref()),
        new_pipeline.page_content.as_ref(),
        old_pipeline.and_then(|pipeline| pipeline.stage_fingerprints.get("content")),
        new_pipeline.stage_fingerprints.get("content"),
        force_stage == Some(MangaPipelineArtifactStage::PageContent),
    )
    .await?;
    persist_page_stage_artifact_if_changed(
        context,
        page,
        MangaPipelineArtifactStage::PageAdaptation,
        old_pipeline.and_then(|pipeline| pipeline.page_adaptation.as_ref()),
        new_pipeline.page_adaptation.as_ref(),
        old_pipeline.and_then(|pipeline| pipeline.stage_fingerprints.get("adaptation")),
        new_pipeline.stage_fingerprints.get("adaptation"),
        force_stage == Some(MangaPipelineArtifactStage::PageAdaptation),
    )
    .await?;
    persist_page_stage_artifact_if_changed(
        context,
        page,
        MangaPipelineArtifactStage::Storyboard,
        old_pipeline.and_then(|pipeline| pipeline.storyboard.as_ref()),
        new_pipeline.storyboard.as_ref(),
        old_storyboard_fingerprint,
        new_pipeline.stage_fingerprints.get("storyboard"),
        force_stage == Some(MangaPipelineArtifactStage::Storyboard),
    )
    .await?;
    persist_page_stage_artifact_if_changed(
        context,
        page,
        MangaPipelineArtifactStage::ReferenceDiscovery,
        old_pipeline.and_then(|pipeline| pipeline.reference_discovery.as_ref()),
        new_pipeline.reference_discovery.as_ref(),
        old_pipeline.and_then(|pipeline| pipeline.stage_fingerprints.get("reference_discovery")),
        new_pipeline.stage_fingerprints.get("reference_discovery"),
        force_stage == Some(MangaPipelineArtifactStage::ReferenceDiscovery),
    )
    .await?;
    persist_page_stage_artifact_if_changed(
        context,
        page,
        MangaPipelineArtifactStage::DrawingPrompt,
        old_pipeline.and_then(|pipeline| pipeline.drawing_prompt.as_ref()),
        new_pipeline.drawing_prompt.as_ref(),
        old_drawing_prompt_fingerprint,
        new_pipeline.stage_fingerprints.get("drawing_prompt"),
        force_stage == Some(MangaPipelineArtifactStage::DrawingPrompt),
    )
    .await?;
    if !legacy_v3 || force_stage == Some(MangaPipelineArtifactStage::Image) {
        persist_page_stage_artifact_if_changed(
            context,
            page,
            MangaPipelineArtifactStage::Image,
            old_pipeline.and_then(|pipeline| pipeline.image.as_ref()),
            new_pipeline.image.as_ref(),
            old_pipeline.and_then(|pipeline| pipeline.stage_fingerprints.get("image")),
            new_pipeline.stage_fingerprints.get("image"),
            force_stage == Some(MangaPipelineArtifactStage::Image),
        )
        .await?;
    }
    let old_bindings = old.as_ref().map(|layout| &layout.reference_bindings);
    persist_page_stage_artifact_if_changed(
        context,
        page,
        MangaPipelineArtifactStage::ReferenceBindings,
        old_bindings,
        (!new.reference_bindings.is_empty()).then_some(&new.reference_bindings),
        None,
        None,
        force_stage == Some(MangaPipelineArtifactStage::ReferenceBindings),
    )
    .await
}

async fn persist_page_stage_artifact_if_changed<T: serde::Serialize + PartialEq>(
    context: &MangaRunContext<'_>,
    page_number: Option<u32>,
    stage: MangaPipelineArtifactStage,
    old_payload: Option<&T>,
    new_payload: Option<&T>,
    old_input_fingerprint: Option<&String>,
    new_input_fingerprint: Option<&String>,
    force: bool,
) -> Result<(), AppError> {
    let Some(payload) = new_payload else {
        return Ok(());
    };
    let input_fingerprint = new_input_fingerprint.cloned().unwrap_or_else(|| {
        source_text_hash(&format!(
            "manual:{}:{}:{}",
            stage.key(),
            context.source_metadata.content_sha256,
            page_number.unwrap_or_default()
        ))
    });
    if !force && old_payload == Some(payload) && old_input_fingerprint == Some(&input_fingerprint) {
        return Ok(());
    }
    let draft = MangaPipelineArtifactDraft {
        project_id: context.project_id,
        chapter_id: context.chapter_id,
        stage,
        page_number,
        input_fingerprint,
        payload: serde_json::to_value(payload)?,
        stale: false,
    };
    if stage == MangaPipelineArtifactStage::ReferenceBindings {
        context
            .service
            .upsert_manga_pipeline_artifact(draft)
            .await?;
    } else {
        context
            .service
            .create_manga_pipeline_artifact(draft)
            .await?;
    }
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
            Some(a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard),
            MangaPagePipelineStage::Storyboard,
        ) | (
            Some(a4n_api_contract::manga::MangaPipelineStopAfter::ReferenceDiscovery),
            MangaPagePipelineStage::ReferenceDiscovery,
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

#[cfg(test)]
mod reference_fingerprint_tests {
    use a4n_domain::manga::{
        MangaBoundReferenceAsset, MangaPipelineArtifactStage, MangaReferenceAssetKind,
        MangaReferenceBindingSummary,
    };
    use a4n_domain::media::GeneratedImageRecord;
    use chrono::Utc;
    use serde_json::json;

    use super::{reference_discovery_is_usable_for_stage, user_reference_image_descriptions};

    #[test]
    fn image_stages_use_the_saved_discovery_result_until_it_is_explicitly_stale() {
        assert!(reference_discovery_is_usable_for_stage(
            Some(MangaPipelineArtifactStage::Image),
            true,
            false,
        ));
        assert!(reference_discovery_is_usable_for_stage(
            Some(MangaPipelineArtifactStage::DrawingPrompt),
            true,
            false,
        ));
        assert!(!reference_discovery_is_usable_for_stage(
            Some(MangaPipelineArtifactStage::Image),
            false,
            true,
        ));
        assert!(!reference_discovery_is_usable_for_stage(None, true, false));
    }

    #[test]
    fn drawing_prompt_fingerprint_uses_only_user_bound_image_descriptions() {
        let now = Utc::now().naive_utc();
        let images = [
            GeneratedImageRecord {
                id: 7,
                project_id: 1,
                image_type: "object_material".into(),
                file_path: "watch.png".into(),
                prompt: "黄铜怀表".into(),
                model: "test".into(),
                width: 1,
                height: 1,
                file_size: None,
                ai_score: None,
                is_favorite: false,
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: now,
            },
            GeneratedImageRecord {
                id: 8,
                project_id: 1,
                image_type: "object_material".into(),
                file_path: "clock.png".into(),
                prompt: "无关钟表".into(),
                model: "test".into(),
                width: 1,
                height: 1,
                file_size: None,
                ai_score: None,
                is_favorite: false,
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: now,
            },
        ];
        let bindings = [MangaReferenceBindingSummary {
            character_name: "林岚".into(),
            bound_asset: None,
            user_added_assets: vec![MangaBoundReferenceAsset {
                asset_kind: MangaReferenceAssetKind::GeneratedImage,
                asset_id: 7,
                image_path: "watch.png".into(),
            }],
        }];

        assert_eq!(
            user_reference_image_descriptions(&bindings, &images),
            json!([{"asset_id": 7, "description": "黄铜怀表"}]),
        );
    }
}
