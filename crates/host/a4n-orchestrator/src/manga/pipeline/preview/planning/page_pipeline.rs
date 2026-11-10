use a4n_domain::manga::{
    MangaPageAdaptationV1, MangaPageContentV1, MangaPageDrawingPromptV2,
    MangaPageReferenceDiscoveryV1, MangaPageSlotsV1, MangaPageV4, MangaPipelineArtifactRecord,
    MangaPipelineArtifactStage,
};
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::adaptation_projection::manga_page_beats_source;
use super::super::super::super::reference_assets::{
    manga_page_reference_discovery_candidates, page_reference_bindings,
    parse_manga_page_reference_discovery, validate_saved_manga_page_reference_discovery,
};
use super::super::super::json::{
    page_drawing_prompt_from_response, parse_manga_page_adaptation, parse_manga_page_slots,
    parse_manga_page_storyboard, resolve_manga_page_slots,
};
use super::super::super::prompt::{
    manga_page_adaptation_user_message, manga_page_drawing_prompt_pipeline_user_message,
    manga_page_reference_discovery_user_message, manga_page_storyboard_user_message,
};
use super::super::super::state::manga_page_pipeline_call_order;
use super::super::call::build_manga_preview_call;
use super::super::contracts::{
    MangaPipelineRequestPreviewStep, MangaPreviewCall, MangaPreviewCallInput, MangaPreviewContext,
};
use crate::manga::layout_rules::manga_page_pipeline_layout;

pub(super) async fn preview_single_page_stage(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction_summary: &Value,
    stage: MangaPipelineArtifactStage,
) -> Result<(), AppError> {
    let page_number = context
        .page_number
        .ok_or_else(|| AppError::InvalidArgument("页级漫画步骤必须指定页码".into()))?;
    let artifacts = context
        .service
        .manga_pipeline_artifacts_by_chapter(context.project_id, context.chapter_id)
        .await?;
    let slots_artifact =
        required_artifact(&artifacts, MangaPipelineArtifactStage::PageSlots, None)?;
    let slots = parse_manga_page_slots(
        &slots_artifact.payload,
        &context.adaptation,
        &context.source_spans,
        "manga_page_slots",
    )?;
    let slots = resolve_manga_page_slots(
        slots,
        &artifacts,
        &context.adaptation,
        &context.source_spans,
    )?;
    required_artifact(
        &artifacts,
        MangaPipelineArtifactStage::PageContent,
        Some(page_number),
    )?;
    let index = slots
        .pages
        .iter()
        .position(|slot| slot.page_number == page_number)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("漫画第 {page_number} 页不存在，无法预览单页步骤"))
        })?;
    let content = &slots.pages[index].content;
    let previous_content = index
        .checked_sub(1)
        .map(|previous| &slots.pages[previous].content);
    let next_content = slots.pages.get(index + 1).map(|page| &page.content);
    let total_pages = slots.pages.len();

    if stage == MangaPipelineArtifactStage::PageAdaptation {
        preview_page_adaptation(
            context,
            steps,
            index,
            page_number,
            content,
            previous_content,
            next_content,
            total_pages,
        )
        .await?;
        return Ok(());
    }

    let adaptation_artifact = required_artifact(
        &artifacts,
        MangaPipelineArtifactStage::PageAdaptation,
        Some(page_number),
    )?;
    let adaptation = parse_manga_page_adaptation(
        &adaptation_artifact.payload,
        content,
        "manga_page_adaptation_plan",
    )?;
    if stage == MangaPipelineArtifactStage::Storyboard {
        preview_page_storyboard(
            context,
            steps,
            index,
            page_number,
            content,
            &adaptation,
            previous_content,
            next_content,
            extraction_summary,
            total_pages,
        )
        .await?;
        return Ok(());
    }

    let storyboard_artifact = required_artifact(
        &artifacts,
        MangaPipelineArtifactStage::Storyboard,
        Some(page_number),
    )?;
    let storyboard = parse_manga_page_storyboard(
        &storyboard_artifact.payload,
        page_number,
        content,
        &adaptation,
        "manga_page_storyboard",
    )?;
    let page_number_db = i32::try_from(page_number)
        .map_err(|_| AppError::InvalidArgument("漫画页码超出 i32 范围".into()))?;
    let page_prompt = context
        .service
        .manga_page_prompts_by_chapter(context.project_id, context.chapter_id)
        .await?
        .into_iter()
        .find(|prompt| prompt.page_number == page_number_db);
    let (current_page_image_id, reference_bindings) = match page_prompt {
        Some(prompt) => {
            let layout = manga_page_pipeline_layout(&prompt).map_err(AppError::InvalidArgument)?;
            (prompt.generated_image_id, layout.reference_bindings)
        }
        None => (None, Vec::new()),
    };

    if stage == MangaPipelineArtifactStage::ReferenceDiscovery {
        preview_page_reference_discovery(
            context,
            steps,
            index,
            page_number,
            content,
            &adaptation,
            &storyboard,
            extraction_summary,
            &reference_bindings,
            current_page_image_id,
        )
        .await?;
        return Ok(());
    }

    let discovery_artifact = required_artifact(
        &artifacts,
        MangaPipelineArtifactStage::ReferenceDiscovery,
        Some(page_number),
    )?;
    let discovery: MangaPageReferenceDiscoveryV1 =
        serde_json::from_value(discovery_artifact.payload.clone())?;
    validate_saved_manga_page_reference_discovery(
        &discovery,
        &context.available_reference_images,
        current_page_image_id,
    )?;
    if stage == MangaPipelineArtifactStage::DrawingPrompt {
        preview_page_drawing_prompt(
            context,
            steps,
            index,
            page_number,
            content,
            &adaptation,
            &storyboard,
            extraction_summary,
            &reference_bindings,
            &discovery,
        )
        .await?;
    } else if stage == MangaPipelineArtifactStage::Image {
        let drawing_prompt: MangaPageDrawingPromptV2 = serde_json::from_value(
            required_artifact(
                &artifacts,
                MangaPipelineArtifactStage::DrawingPrompt,
                Some(page_number),
            )?
            .payload
            .clone(),
        )?;
        if drawing_prompt.page_number != page_number
            || drawing_prompt.image_prompt.trim().is_empty()
        {
            return Err(AppError::InvalidState {
                current: "manga_drawing_prompt_invalid".into(),
                operation: format!("请先修正第 {page_number} 页绘图提示词"),
            });
        }
    }
    Ok(())
}

fn required_artifact(
    artifacts: &[MangaPipelineArtifactRecord],
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> Result<&MangaPipelineArtifactRecord, AppError> {
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact.stage == stage && artifact.page_number == page_number)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("manga_stage_artifact_missing:{}", stage.key()),
            operation: format!("当前阶段缺少前置产物：{}", stage.key()),
        })?;
    if artifact.stale {
        return Err(AppError::InvalidState {
            current: format!("manga_stage_artifact_stale:{}", stage.key()),
            operation: format!("前置产物 {} 已过期，请先重跑对应步骤", stage.key()),
        });
    }
    Ok(artifact)
}

pub(super) async fn preview_page_pipeline(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    slots: &MangaPageSlotsV1,
    extraction_summary: &Value,
) -> Result<(), AppError> {
    let total_pages = slots.pages.len();
    if let Some(page_number) = context.page_number
        && !slots
            .pages
            .iter()
            .any(|slot| slot.page_number == page_number)
    {
        return Err(AppError::InvalidArgument(format!(
            "漫画第 {page_number} 页不存在，无法预览单页流水线"
        )));
    }
    for (index, slot) in slots.pages.iter().enumerate() {
        let page_number = slot.page_number;
        if context
            .page_number
            .is_some_and(|selected| selected != page_number)
        {
            continue;
        }
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
            Some(a4n_api_contract::manga::MangaPipelineStopAfter::Storyboard)
        ) {
            continue;
        }
        let reference_bindings = page_reference_bindings(&storyboard, &context.reference_assets)
            .map_err(AppError::InvalidArgument)?;
        let discovery = match preview_page_reference_discovery(
            context,
            steps,
            index,
            page_number,
            content,
            &adaptation,
            &storyboard,
            extraction_summary,
            &reference_bindings,
            None,
        )
        .await?
        {
            Some(discovery) => discovery,
            None => continue,
        };
        if context.stop_after
            == Some(a4n_api_contract::manga::MangaPipelineStopAfter::ReferenceDiscovery)
        {
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
            &reference_bindings,
            &discovery,
        )
        .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn preview_page_reference_discovery(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    index: usize,
    page_number: u32,
    content: &MangaPageContentV1,
    adaptation: &MangaPageAdaptationV1,
    storyboard: &MangaPageV4,
    extraction_summary: &Value,
    reference_bindings: &[a4n_domain::manga::MangaReferenceBindingSummary],
    current_page_image_id: Option<i64>,
) -> Result<Option<MangaPageReferenceDiscoveryV1>, AppError> {
    let key = format!("manga_page_reference_discovery_page_{page_number}");
    let title = format!("第 {page_number} 页参考图发现");
    let (candidate_assets, candidates) = manga_page_reference_discovery_candidates(
        &context.available_reference_images,
        current_page_image_id,
        reference_bindings,
    );
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_reference_discovery",
        workflow_run_id: context.workflow_run_id,
        call_key: a4n_domain::manga::MangaModelCall::ReferenceDiscovery(page_number)
            .call_key(context.chapter_id),
        call_order: manga_page_pipeline_call_order(index, 2)?,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_reference_discovery_user_message(
            page_number,
            storyboard,
            content,
            adaptation,
            extraction_summary,
            candidate_assets,
            &context.source_markers,
        )?,
        temperature: 0.2,
    })
    .await?;
    match call {
        MangaPreviewCall::Restored { parsed_output } => {
            steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                &key,
                &title,
                "manga_page_reference_discovery",
            ));
            parse_manga_page_reference_discovery(&parsed_output, page_number, &candidates).map(Some)
        }
        MangaPreviewCall::Ready(request) => {
            steps.push(MangaPipelineRequestPreviewStep::ready(
                &key,
                &title,
                "manga_page_reference_discovery",
                None,
                *request,
            ));
            Ok(None)
        }
    }
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
    let title = format!("第 {page_number} 页改编计划");
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_adaptation_plan",
        workflow_run_id: context.workflow_run_id,
        call_key: a4n_domain::manga::MangaModelCall::PageAdaptation(page_number)
            .call_key(context.chapter_id),
        call_order: manga_page_pipeline_call_order(index, 0)?,
        restore_from_checkpoint: context.resume_from_checkpoint,
        prompt_renderer: context.prompt_renderer,
        project_id: context.project_id,
        variables: &context.variables,
        user_content: manga_page_adaptation_user_message(
            page_number,
            total_pages,
            content,
            manga_page_beats_source(&context.adaptation, &content.beat_refs),
            previous_content,
            next_content,
            &context.source_markers,
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
    let title = format!("第 {page_number} 页分镜");
    let adaptation_beats = manga_page_beats_source(&context.adaptation, &content.beat_refs);
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_storyboard",
        workflow_run_id: context.workflow_run_id,
        call_key: a4n_domain::manga::MangaModelCall::Storyboard(page_number)
            .call_key(context.chapter_id),
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
            &context.source_spans,
            &context.chapter_content,
            &context.source_markers,
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
    reference_bindings: &[a4n_domain::manga::MangaReferenceBindingSummary],
    discovery: &MangaPageReferenceDiscoveryV1,
) -> Result<(), AppError> {
    let key = format!("manga_page_drawing_prompt_page_{page_number}");
    let title = format!("第 {page_number} 页绘图提示词");
    let call = build_manga_preview_call(MangaPreviewCallInput {
        service: context.service,
        prompt_name: "manga_page_drawing_prompt",
        workflow_run_id: context.workflow_run_id,
        call_key: a4n_domain::manga::MangaModelCall::DrawingPrompt(page_number)
            .call_key(context.chapter_id),
        call_order: manga_page_pipeline_call_order(index, 3)?,
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
            reference_bindings,
            &discovery.requirements,
            &discovery.references,
            &context.available_reference_images,
            &context.source_markers,
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
