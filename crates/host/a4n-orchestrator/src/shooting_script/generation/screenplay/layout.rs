use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    shooting_script::{
        context::{shooting_context_request, visual_unit_prompt_context},
        contracts::{
            CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT, CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            ChapterScreenplaySkeletonV1, PreparedChapterShootingScriptRequest,
            ScreenplayLayoutUnitDetailV2, ScreenplayLayoutUnitV1, ScreenplaySkeletonUnitV1,
            ShootingScriptProgressReporter, ShootingScriptStreamReporter,
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
    super::super::super::contracts::ShootingScriptStageResult<ScreenplayLayoutUnitV1>,
    AppError,
> {
    let planned = prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?;
    if !prepared.shot_ids.is_empty()
        && let Some(scene) = prepared.existing.as_ref().and_then(|existing| {
            existing.script.scenes.iter().find(|scene| {
                scene.scene_identity == unit.scene_identity
                    && scene.shots.iter().any(|shot| {
                        shot.visual_constraints
                            .as_ref()
                            .is_some_and(|old| old.scene_binding == planned.scene_binding)
                    })
            })
        })
    {
        return Ok(super::super::super::contracts::ShootingScriptStageResult {
            restored: true,
            output: ScreenplayLayoutUnitV1 {
                scene_identity: scene.scene_identity.clone(),
                sequence_index: scene.sequence_index,
                narrative_unit_index: scene.narrative_unit_index,
                timeline_position: scene.timeline_position,
                intercut_group_id: scene.intercut_group_id.clone(),
                editing_mode: scene.editing_mode,
                transition_contracts: scene.transition_contracts.clone(),
                title: scene.title.clone(),
                slugline: scene.slugline.clone(),
                source_span_ids: scene.source_span_ids.clone(),
                participants: scene.participants.clone(),
                dramatic_question: scene.dramatic_question.clone(),
                scene_objective: scene.scene_objective.clone(),
                scene_obstacle: scene.scene_obstacle.clone(),
                entry_state: scene.entry_state.clone(),
                exit_state: scene.exit_state.clone(),
            },
        });
    }
    let unit_progress = layout_unit_stream_progress(unit_offset, total_unit_count);
    let request = screenplay_layout_unit_request(prepared, workflow_run_id, skeleton, unit)?;
    let stage = orchestrator
        .run_shooting_script_stage::<ScreenplayLayoutUnitDetailV2>(
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
                validate_layout_unit_detail(
                    output,
                    unit.narrative_unit_index,
                    total_unit_count,
                    &unit.source_span_ids,
                )
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
    Ok(super::super::super::contracts::ShootingScriptStageResult {
        restored: stage.restored,
        output: super::assembly::assemble_screenplay_layout_unit(unit, stage.output),
    })
}

fn screenplay_layout_unit_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    skeleton: &ChapterScreenplaySkeletonV1,
    unit: &ScreenplaySkeletonUnitV1,
) -> Result<a4n_llm_provider_contract::provider::ChatRequest, AppError> {
    let visual_unit = prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?;
    let mut skeleton_context = serde_json::to_value(skeleton)?;
    for item in skeleton_context["units"]
        .as_array_mut()
        .expect("skeleton units")
    {
        item.as_object_mut()
            .expect("skeleton unit")
            .remove("source_span_ids");
    }
    skeleton_context["locked_visual_unit"] = visual_unit_prompt_context(visual_unit)?;
    let unit_index = (unit.narrative_unit_index - 1) as usize;
    let units = &prepared.visual_plan_projection.narrative_units;
    skeleton_context["previous_unit_end"] = serde_json::to_value(
        unit_index
            .checked_sub(1)
            .and_then(|index| units.get(index))
            .and_then(|unit| unit.shots.last()),
    )?;
    skeleton_context["next_unit_start"] = serde_json::to_value(
        units
            .get(unit_index + 1)
            .and_then(|unit| unit.shots.first()),
    )?;
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SCREENPLAY_LAYOUT_UNIT_PROMPT,
        format!("screenplay_layout_unit:{}", unit.narrative_unit_index),
        prepared.layout_unit_system_prompt.clone(),
        (&unit.source_span_ids, &visual_unit.scene_binding),
        Some(("shooting_layout_skeleton", &skeleton_context)),
        format!(
            "只细化 narrative_unit_index={}。",
            unit.narrative_unit_index
        ),
    )
}
