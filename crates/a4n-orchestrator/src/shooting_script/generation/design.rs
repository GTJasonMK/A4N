use std::collections::HashSet;

use a4n_domain::llm::llm_token_profiles;
use a4n_domain::shooting_script::{ShootingSceneV1, ShootingShotV1};
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

use super::super::{
    context::shooting_context_request,
    contracts::{
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT, CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        PreparedChapterShootingScriptRequest, ScreenplayPlanUnit, ShootingScriptProgressReporter,
        ShootingScriptStreamReporter,
    },
    progress::design_unit_stream_progress,
};

pub(super) async fn generate_shooting_scene(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    unit: &ScreenplayPlanUnit,
    unit_offset: usize,
    total_unit_count: usize,
    progress: ShootingScriptProgressReporter<'_>,
    stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
    cancel_token: Option<&CancellationToken>,
) -> Result<super::super::contracts::ShootingScriptStageResult<ShootingSceneV1>, AppError> {
    let step_key = format!("shooting_design_unit:{}", unit.narrative_unit_index);
    let request = design_unit_request(prepared, workflow_run_id, unit)?;
    let unit_progress = design_unit_stream_progress(unit_offset, total_unit_count);
    progress(
        unit_progress.span.start(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!("正在设计拍摄单元 {}/{}", unit_offset + 1, total_unit_count),
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
            |output| validate_design_unit(output, unit, prepared),
        )
        .await?;
    progress(
        unit_progress.span.end(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!(
            "拍摄单元 {}/{} {}，正在校验",
            unit_offset + 1,
            total_unit_count,
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
    let planned_unit = planned_visual_unit(prepared, unit)?;
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
        format!("shooting_design_unit:{}", unit.narrative_unit_index),
        prepared.design_unit_system_prompt.clone(),
        Some((
            "shooting_visual_design_input",
            &serde_json::json!({
                "screenplay_plan_unit": unit,
                "locked_visual_unit": planned_unit,
            }),
        )),
        format!(
            "只设计 narrative_unit_index={} 的 ShootingSceneV1。shots 必须与 locked_visual_unit.shots 数量和顺序一致；不得改变视觉镜头 ID、时间范围、source_span_ids、user_addition_ids 或关键帧/事件约束。",
            unit.narrative_unit_index,
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
            |output| validate_design_unit(output, unit, prepared),
        )
        .await
}

fn validate_design_unit(
    scene: &ShootingSceneV1,
    unit: &ScreenplayPlanUnit,
    prepared: &PreparedChapterShootingScriptRequest,
) -> Result<(), AppError> {
    if scene.narrative_unit_index != unit.narrative_unit_index || scene.shots.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元 {} 的身份无效或镜头为空",
            unit.narrative_unit_index
        )));
    }
    let planned = planned_visual_unit(prepared, unit)?;
    if scene.shots.len() != planned.shots.len() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计 unit {} 必须生成 {} 个视觉方案镜头，实际 {} 个",
            unit.narrative_unit_index,
            planned.shots.len(),
            scene.shots.len()
        )));
    }
    for (shot, planned_shot) in scene.shots.iter().zip(&planned.shots) {
        if shot.source_span_ids != planned_shot.source_span_ids {
            return Err(AppError::InvalidArgument(format!(
                "拍摄设计 unit {} 的镜头来源顺序偏离 confirmed 导演编排",
                unit.narrative_unit_index
            )));
        }
    }
    validate_design_scene_content_against_plan_unit(scene, unit)
}

fn planned_visual_unit<'a>(
    prepared: &'a PreparedChapterShootingScriptRequest,
    unit: &ScreenplayPlanUnit,
) -> Result<&'a a4n_domain::chapter_visual_plan::VisualNarrativeUnitV1, AppError> {
    prepared
        .visual_plan_projection
        .narrative_units
        .get(usize::try_from(unit.narrative_unit_index - 1).map_err(|_| {
            AppError::InvalidArgument("narrative_unit_index 超出 usize 范围".into())
        })?)
        .filter(|planned| planned.id == unit.scene_identity)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "拍摄设计 unit {} 找不到对应的 confirmed 视觉单元",
                unit.narrative_unit_index
            ))
        })
}

fn validate_design_scene_content_against_plan_unit(
    scene: &ShootingSceneV1,
    planned_unit: &ScreenplayPlanUnit,
) -> Result<(), AppError> {
    let allowed_sources = planned_unit
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let participants = planned_unit
        .participants
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if !optional_actor_is_allowed(scene.focal_character.as_deref(), &participants) {
        return Err(AppError::InvalidArgument(format!(
            "镜头设计 unit {} 的 focal_character 不属于 participants",
            scene.narrative_unit_index
        )));
    }
    if !optional_actor_is_allowed(scene.viewpoint.anchor_character.as_deref(), &participants) {
        return Err(AppError::InvalidArgument(format!(
            "镜头设计 unit {} 的 viewpoint.anchor_character 不属于 participants",
            scene.narrative_unit_index
        )));
    }
    for shot in &scene.shots {
        validate_shot_sources(
            scene.narrative_unit_index,
            shot,
            &allowed_sources,
            &participants,
        )?;
    }
    let required_beats = scene
        .shots
        .iter()
        .flat_map(|shot| &shot.performance_beats)
        .filter(|beat| beat.required)
        .collect::<Vec<_>>();
    let required_beat_sources = required_source_ids(
        required_beats
            .iter()
            .map(|beat| beat.source_span_ids.as_slice()),
    );
    for fact in planned_unit
        .performance_facts
        .iter()
        .filter(|fact| fact.required)
    {
        if !fact
            .source_span_ids
            .iter()
            .any(|id| required_beat_sources.contains(id.as_str()))
        {
            return Err(AppError::InvalidArgument(format!(
                "镜头设计 unit {} 的 required performance fact {} 未被 required beat 覆盖",
                scene.narrative_unit_index, fact.fact_index
            )));
        }
    }
    Ok(())
}

fn validate_shot_sources(
    narrative_unit_index: u32,
    shot: &ShootingShotV1,
    allowed_sources: &HashSet<&str>,
    participants: &HashSet<&str>,
) -> Result<(), AppError> {
    if shot
        .source_span_ids
        .iter()
        .any(|id| !allowed_sources.contains(id.as_str()))
    {
        return Err(AppError::InvalidArgument(format!(
            "镜头设计 unit {narrative_unit_index} 的 shot {} 引用了当前单元范围外的 source span",
            shot.shot_index
        )));
    }
    let shot_sources = shot
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    for beat in &shot.performance_beats {
        if !optional_actor_is_allowed(beat.actor.as_deref(), participants) {
            return Err(AppError::InvalidArgument(format!(
                "镜头设计 unit {narrative_unit_index} 的 shot {} beat {} actor 不属于 participants",
                shot.shot_index, beat.beat_index
            )));
        }
        if beat
            .source_span_ids
            .iter()
            .any(|id| !shot_sources.contains(id.as_str()))
        {
            return Err(AppError::InvalidArgument(format!(
                "镜头设计 unit {narrative_unit_index} 的 shot {} beat {} 来源范围无效",
                shot.shot_index, beat.beat_index
            )));
        }
    }
    Ok(())
}

fn required_source_ids<'a, I>(source_lists: I) -> HashSet<&'a str>
where
    I: IntoIterator<Item = &'a [String]>,
{
    source_lists
        .into_iter()
        .flat_map(|sources| sources.iter().map(String::as_str))
        .collect()
}

fn optional_actor_is_allowed(actor: Option<&str>, participants: &HashSet<&str>) -> bool {
    actor.is_none_or(|actor| participants.contains(actor))
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
    use std::collections::HashSet;

    use super::{optional_actor_is_allowed, required_source_ids, shooting_design_call_order};

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

    #[test]
    fn fact_coverage_is_counted_by_beat_overlap_not_matching_span_count() {
        let fact_sources = ["span-1".to_string(), "span-2".to_string()];
        let beat_sources = [
            "span-1".to_string(),
            "span-2".to_string(),
            "span-3".to_string(),
        ];

        let required_sources = required_source_ids([beat_sources.as_slice()]);
        assert!(
            fact_sources
                .iter()
                .any(|source| required_sources.contains(source.as_str()))
        );
    }

    #[test]
    fn actor_must_be_one_exact_planned_participant() {
        let participants = HashSet::from(["林清辞", "苏小曼"]);

        assert!(optional_actor_is_allowed(Some("苏小曼"), &participants));
        assert!(!optional_actor_is_allowed(
            Some("苏小曼与林清辞"),
            &participants,
        ));
        assert!(optional_actor_is_allowed(None, &participants));
    }
}
