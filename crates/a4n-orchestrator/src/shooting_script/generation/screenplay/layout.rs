use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    shooting_script::{
        context::shooting_context_request,
        contracts::{
            CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT, CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            ChapterScreenplaySkeletonV1, PreparedChapterShootingScriptRequest,
            ScreenplayLayoutUnitDetailV1, ScreenplaySkeletonUnitV1, ShootingScriptProgressReporter,
            ShootingScriptStreamReporter,
        },
        progress::{SKELETON_STREAM_SPAN, layout_unit_stream_progress},
    },
};

use super::validation::{validate_layout_unit_detail, validate_skeleton};

pub(in crate::shooting_script::generation) fn generate_screenplay_skeleton(
    prepared: &PreparedChapterShootingScriptRequest,
    progress: ShootingScriptProgressReporter<'_>,
) -> Result<ChapterScreenplaySkeletonV1, AppError> {
    progress(
        1,
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        "正在生成或恢复全章叙事骨架",
    )?;
    let skeleton = skeleton_from_visual_plan(prepared)?;
    validate_skeleton(&skeleton, prepared)?;
    progress(
        SKELETON_STREAM_SPAN.end(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        "已按 confirmed 导演编排锁定全章骨架，开始逐单元细化",
    )?;
    Ok(skeleton)
}

pub(in crate::shooting_script) fn restore_screenplay_skeleton(
    prepared: &PreparedChapterShootingScriptRequest,
) -> Result<ChapterScreenplaySkeletonV1, AppError> {
    let skeleton = skeleton_from_visual_plan(prepared)?;
    validate_skeleton(&skeleton, prepared)?;
    Ok(skeleton)
}

fn skeleton_from_visual_plan(
    prepared: &PreparedChapterShootingScriptRequest,
) -> Result<ChapterScreenplaySkeletonV1, AppError> {
    let units = prepared
        .visual_plan_projection
        .narrative_units
        .iter()
        .enumerate()
        .map(|(offset, unit)| {
            Ok(ScreenplaySkeletonUnitV1 {
                scene_identity: unit.id.clone(),
                sequence_index: unit.sequence_index,
                narrative_unit_index: u32::try_from(offset + 1).map_err(|_| {
                    AppError::InvalidArgument("视觉编排叙事单元数量超出 u32 范围".into())
                })?,
                timeline_position: unit.timeline_position,
                intercut_group_id: unit.intercut_group_id.clone(),
                editing_mode: unit.editing_mode,
                source_span_ids: unit.source_span_ids.clone(),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(ChapterScreenplaySkeletonV1 {
        schema_version: "chapter_screenplay_skeleton_v1".into(),
        directing_statement: prepared.visual_plan_projection.directing_statement.clone(),
        units,
    })
}

pub(in crate::shooting_script::generation) async fn generate_screenplay_layout_unit(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    skeleton: &ChapterScreenplaySkeletonV1,
    unit: &ScreenplaySkeletonUnitV1,
    unit_offset: usize,
    total_unit_count: usize,
    progress: ShootingScriptProgressReporter<'_>,
    stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
    cancel_token: Option<&CancellationToken>,
) -> Result<
    super::super::super::contracts::ShootingScriptStageResult<ScreenplayLayoutUnitDetailV1>,
    AppError,
> {
    let unit_progress = layout_unit_stream_progress(unit_offset, total_unit_count);
    let request = screenplay_layout_unit_request(prepared, workflow_run_id, skeleton, unit)?;
    let stage = orchestrator
        .run_shooting_script_stage::<ScreenplayLayoutUnitDetailV1>(
            workflow_run_id,
            &format!("screenplay_layout_unit:{}", unit.narrative_unit_index),
            i32::try_from(
                unit_offset
                    .checked_mul(3)
                    .and_then(|value| value.checked_add(2))
                    .ok_or_else(|| AppError::InvalidArgument("布局单元数量超限".into()))?,
            )
            .map_err(|_| AppError::InvalidArgument("布局单元数量超限".into()))?,
            request,
            "叙事单元布局细化",
            unit_progress,
            CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            progress,
            stream_reporter,
            cancel_token,
            |output| {
                validate_layout_unit_detail(output, unit.narrative_unit_index, total_unit_count)
            },
        )
        .await?;
    progress(
        unit_progress.span.end(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!(
            "布局单元 {}/{} {}",
            unit_offset + 1,
            total_unit_count,
            if stage.restored {
                "已从调用断点恢复"
            } else {
                "细化完成"
            }
        ),
    )?;
    Ok(stage)
}

pub(in crate::shooting_script) async fn restore_screenplay_layout_unit(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    skeleton: &ChapterScreenplaySkeletonV1,
    unit: &ScreenplaySkeletonUnitV1,
) -> Result<Option<ScreenplayLayoutUnitDetailV1>, AppError> {
    let request = screenplay_layout_unit_request(prepared, workflow_run_id, skeleton, unit)?;
    orchestrator
        .restore_completed_shooting_script_stage(
            workflow_run_id,
            &format!("screenplay_layout_unit:{}", unit.narrative_unit_index),
            &request,
            |output| {
                validate_layout_unit_detail(output, unit.narrative_unit_index, skeleton.units.len())
            },
        )
        .await
}

fn screenplay_layout_unit_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    skeleton: &ChapterScreenplaySkeletonV1,
    unit: &ScreenplaySkeletonUnitV1,
) -> Result<a4n_llm_provider_contract::provider::ChatRequest, AppError> {
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT,
        format!("screenplay_layout_unit:{}", unit.narrative_unit_index),
        prepared.layout_unit_system_prompt.clone(),
        Some(("shooting_layout_skeleton", skeleton)),
        format!(
            "只细化 narrative_unit_index={}。",
            unit.narrative_unit_index
        ),
    )
}
