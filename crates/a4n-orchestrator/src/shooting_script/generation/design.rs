use a4n_core::{
    config::app_config::llm_token_profiles, error::AppError, shooting_script::ShootingSceneV1,
};
use a4n_llm::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

use super::super::{
    context::shooting_context_request,
    contracts::{
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT, ChapterScreenplayPlan,
        PreparedChapterShootingScriptRequest, ScreenplayPlanUnit, ShootingScriptProgressReporter,
    },
    progress::{DESIGN_STREAM_SPAN, design_unit_stream_progress},
    screenplay::validate_design_scene_content_against_plan_unit,
};

pub(super) async fn generate_shooting_scenes(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    plan: &ChapterScreenplayPlan,
    progress_total: u32,
    progress: ShootingScriptProgressReporter<'_>,
    cancel_token: Option<&CancellationToken>,
) -> Result<Vec<ShootingSceneV1>, AppError> {
    let restored_designs =
        restorable_design_prefix(orchestrator, workflow_run_id, prepared, plan).await?;
    let design_start_progress = restored_design_progress(restored_designs, plan.units.len());
    progress(
        design_start_progress,
        progress_total,
        if restored_designs == 0 {
            "剧本改编已完成，开始逐单元拍摄设计".to_string()
        } else {
            format!(
                "已确认可恢复 {restored_designs}/{} 个拍摄设计单元",
                plan.units.len()
            )
        }
        .as_str(),
    )?;

    let mut scenes = Vec::with_capacity(plan.units.len());
    for (unit_offset, unit) in plan.units.iter().enumerate() {
        let step_key = format!("shooting_design_unit:{}", unit.narrative_unit_index);
        let request =
            design_unit_request(prepared, workflow_run_id, plan, unit.narrative_unit_index)?;
        let unit_progress = design_unit_stream_progress(unit_offset, plan.units.len());
        let stage_progress = |current: u32, total: u32, message: &str| {
            progress(current.max(design_start_progress), total, message)
        };
        progress(
            unit_progress.span.start().max(design_start_progress),
            progress_total,
            &format!("正在设计拍摄单元 {}/{}", unit_offset + 1, plan.units.len()),
        )?;
        let stage = orchestrator
            .run_shooting_script_stage::<ShootingSceneV1>(
                workflow_run_id,
                &step_key,
                shooting_design_call_order(plan.units.len(), unit_offset)?,
                request,
                "拍摄设计单元",
                unit_progress,
                progress_total,
                &stage_progress,
                cancel_token,
                |output| validate_design_unit(output, unit),
            )
            .await?;
        progress(
            unit_progress.span.end().max(design_start_progress),
            progress_total,
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
        scenes.push(stage.output);
    }
    Ok(scenes)
}

fn design_unit_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    plan: &ChapterScreenplayPlan,
    narrative_unit_index: u32,
) -> Result<ChatRequest, AppError> {
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
        format!("shooting_design_unit:{narrative_unit_index}"),
        prepared.design_unit_system_prompt.clone(),
        Some(("shooting_screenplay_plan", plan)),
        format!("只设计 narrative_unit_index={narrative_unit_index} 的 ShootingSceneV1。"),
        llm_token_profiles::EXTRA_LONG.into(),
    )
}

async fn restorable_design_prefix(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    plan: &ChapterScreenplayPlan,
) -> Result<usize, AppError> {
    let mut restored = 0;
    for unit in &plan.units {
        let step_key = format!("shooting_design_unit:{}", unit.narrative_unit_index);
        let request =
            design_unit_request(prepared, workflow_run_id, plan, unit.narrative_unit_index)?;
        let output = orchestrator
            .restore_completed_shooting_script_stage::<ShootingSceneV1>(
                workflow_run_id,
                &step_key,
                &request,
                "拍摄设计单元",
                |output| validate_design_unit(output, unit),
            )
            .await?;
        if output.is_none() {
            break;
        }
        restored += 1;
    }
    Ok(restored)
}

fn restored_design_progress(restored_designs: usize, design_count: usize) -> u32 {
    if restored_designs == 0 {
        DESIGN_STREAM_SPAN.start()
    } else {
        design_unit_stream_progress(restored_designs - 1, design_count)
            .span
            .end()
    }
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

fn shooting_design_call_order(unit_count: usize, unit_offset: usize) -> Result<i32, AppError> {
    let order = unit_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(unit_offset))
        .and_then(|value| value.checked_add(2))
        .ok_or_else(|| AppError::InvalidArgument("拍摄设计单元数量超出 usize 范围".into()))?;
    i32::try_from(order)
        .map_err(|_| AppError::InvalidArgument("拍摄设计单元数量超出 i32 范围".into()))
}

#[cfg(test)]
mod tests {
    use super::{restored_design_progress, shooting_design_call_order};

    #[test]
    fn design_call_order_follows_all_layout_and_adaptation_calls() {
        assert_eq!(
            shooting_design_call_order(3, 0).expect("first design order"),
            8
        );
        assert_eq!(
            shooting_design_call_order(3, 2).expect("last design order"),
            10
        );
    }

    #[test]
    fn restored_design_progress_uses_completed_prefix_floor() {
        assert_eq!(restored_design_progress(0, 20), 50);
        assert_eq!(restored_design_progress(5, 20), 60);
        assert_eq!(restored_design_progress(20, 20), 90);
    }
}
