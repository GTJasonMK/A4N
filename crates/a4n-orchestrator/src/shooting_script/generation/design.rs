use a4n_core::{
    config::app_config::llm_token_profiles, error::AppError, shooting_script::ShootingSceneV1,
};
use a4n_llm::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

use super::super::{
    context::shooting_context_request,
    contracts::{
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT, CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        ChapterScreenplayPlan, PreparedChapterShootingScriptRequest, ScreenplayPlanUnit,
        ShootingScriptProgressReporter, ShootingScriptStreamReporter,
    },
    progress::design_unit_stream_progress,
    screenplay::validate_design_scene_content_against_plan_unit,
};

pub(super) async fn generate_shooting_scene(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    plan: &ChapterScreenplayPlan,
    unit_offset: usize,
    progress: ShootingScriptProgressReporter<'_>,
    stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
    cancel_token: Option<&CancellationToken>,
) -> Result<super::super::contracts::ShootingScriptStageResult<ShootingSceneV1>, AppError> {
    let unit = plan.units.get(unit_offset).ok_or_else(|| {
        AppError::InvalidArgument(format!("不存在第 {} 个拍摄设计单元", unit_offset + 1))
    })?;
    let step_key = format!("shooting_design_unit:{}", unit.narrative_unit_index);
    let request = design_unit_request(prepared, workflow_run_id, unit)?;
    let unit_progress = design_unit_stream_progress(unit_offset, plan.units.len());
    progress(
        unit_progress.span.start(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!("正在设计拍摄单元 {}/{}", unit_offset + 1, plan.units.len()),
    )?;
    let stage = orchestrator
        .run_shooting_script_stage::<ShootingSceneV1>(
            workflow_run_id,
            &step_key,
            shooting_design_call_order(unit_offset)?,
            request,
            "拍摄设计单元",
            unit_progress,
            CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            progress,
            stream_reporter,
            cancel_token,
            |output| validate_design_unit(output, unit),
        )
        .await?;
    progress(
        unit_progress.span.end(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!(
            "拍摄单元 {}/{} {}，正在校验",
            unit_offset + 1,
            plan.units.len(),
            if stage.restored {
                "已从断点恢复"
            } else {
                "已生成"
            }
        ),
    )?;
    Ok(stage)
}

fn design_unit_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    unit: &ScreenplayPlanUnit,
) -> Result<ChatRequest, AppError> {
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
        format!("shooting_design_unit:{}", unit.narrative_unit_index),
        prepared.design_unit_system_prompt.clone(),
        Some(("shooting_screenplay_plan_unit", unit)),
        format!(
            "只设计 narrative_unit_index={} 的 ShootingSceneV1。",
            unit.narrative_unit_index
        ),
        llm_token_profiles::EXTRA_LONG.into(),
    )
}

pub(in crate::shooting_script) async fn restore_shooting_scene(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    unit: &ScreenplayPlanUnit,
) -> Result<Option<ShootingSceneV1>, AppError> {
    let request = design_unit_request(prepared, workflow_run_id, unit)?;
    orchestrator
        .restore_completed_shooting_script_stage(
            workflow_run_id,
            &format!("shooting_design_unit:{}", unit.narrative_unit_index),
            &request,
            "拍摄设计单元",
            |output| validate_design_unit(output, unit),
        )
        .await
}

fn validate_design_unit(
    scene: &ShootingSceneV1,
    unit: &ScreenplayPlanUnit,
) -> Result<(), AppError> {
    if scene.narrative_unit_index != unit.narrative_unit_index || scene.shots.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元 {} 的身份无效或镜头为空",
            unit.narrative_unit_index
        )));
    }
    validate_design_scene_content_against_plan_unit(scene, unit)
}

fn shooting_design_call_order(unit_offset: usize) -> Result<i32, AppError> {
    let order = unit_offset
        .checked_mul(3)
        .and_then(|value| value.checked_add(4))
        .ok_or_else(|| AppError::InvalidArgument("拍摄设计单元数量超出 usize 范围".into()))?;
    i32::try_from(order)
        .map_err(|_| AppError::InvalidArgument("拍摄设计单元数量超出 i32 范围".into()))
}

#[cfg(test)]
mod tests {
    use super::shooting_design_call_order;

    #[test]
    fn design_call_order_follows_layout_and_adaptation_for_each_unit() {
        assert_eq!(
            shooting_design_call_order(0).expect("first design order"),
            4
        );
        assert_eq!(
            shooting_design_call_order(2).expect("last design order"),
            10
        );
    }
}
