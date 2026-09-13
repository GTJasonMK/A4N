use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    shooting_script::{
        context::shooting_context_request,
        contracts::{
            CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT, CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            PreparedChapterShootingScriptRequest, ScreenplayLayoutUnitV1,
            ScreenplayUnitAdaptationV1, ShootingScriptProgressReporter,
            ShootingScriptStreamReporter,
        },
        progress::unit_adaptation_stream_progress,
    },
};

use super::validation::validate_unit_adaptation;

pub(in crate::shooting_script::generation) async fn generate_screenplay_adaptation_unit(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    unit: &ScreenplayLayoutUnitV1,
    unit_offset: usize,
    total_unit_count: usize,
    progress: ShootingScriptProgressReporter<'_>,
    stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
    cancel_token: Option<&CancellationToken>,
) -> Result<
    super::super::super::contracts::ShootingScriptStageResult<ScreenplayUnitAdaptationV1>,
    AppError,
> {
    let unit_progress = unit_adaptation_stream_progress(unit_offset, total_unit_count);
    let request = unit_adaptation_request(prepared, workflow_run_id, unit)?;
    let stage = orchestrator
        .run_shooting_script_stage::<ScreenplayUnitAdaptationV1>(
            workflow_run_id,
            &format!("screenplay_unit_adaptation:{}", unit.narrative_unit_index),
            i32::try_from(
                unit_offset
                    .checked_mul(3)
                    .and_then(|value| value.checked_add(3))
                    .ok_or_else(|| AppError::InvalidArgument("改编单元数量超限".into()))?,
            )
            .map_err(|_| AppError::InvalidArgument("改编单元数量超限".into()))?,
            request,
            "叙事单元剧本改编",
            unit_progress,
            CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            progress,
            stream_reporter,
            cancel_token,
            |output| {
                validate_unit_adaptation(
                    output,
                    unit,
                    prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?,
                )
            },
        )
        .await?;
    progress(
        unit_progress.span.end(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!(
            "叙事单元 {}/{} {}",
            unit_offset + 1,
            total_unit_count,
            if stage.restored {
                "已从调用断点恢复"
            } else {
                "表演事实完成"
            }
        ),
    )?;
    Ok(stage)
}

pub(in crate::shooting_script) async fn restore_screenplay_adaptation_unit(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    unit: &ScreenplayLayoutUnitV1,
) -> Result<Option<ScreenplayUnitAdaptationV1>, AppError> {
    let request = unit_adaptation_request(prepared, workflow_run_id, unit)?;
    orchestrator
        .restore_completed_shooting_script_stage(
            workflow_run_id,
            &format!("screenplay_unit_adaptation:{}", unit.narrative_unit_index),
            &request,
            |output| {
                validate_unit_adaptation(
                    output,
                    unit,
                    prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?,
                )
            },
        )
        .await
}

fn unit_adaptation_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    unit: &ScreenplayLayoutUnitV1,
) -> Result<ChatRequest, AppError> {
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT,
        format!("screenplay_unit_adaptation:{}", unit.narrative_unit_index),
        prepared.unit_adaptation_system_prompt.clone(),
        Some((
            "shooting_screenplay_layout_unit",
            &serde_json::json!({
                "unit": unit,
                "locked_visual_unit": prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?,
            }),
        )),
        format!(
            "只改编以下目标 unit 的 performance_facts，以 locked_visual_unit 的镜头 prompt 为导演事实来源，引用 ID 为 director:{{segment_id}}；正文只作为相关上下文，不得把整章动作重复塞入每个单元：\n{}",
            serde_json::to_string(unit)?
        ),
    )
}
