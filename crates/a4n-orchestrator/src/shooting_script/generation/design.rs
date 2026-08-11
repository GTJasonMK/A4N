use std::collections::HashSet;

use a4n_core::{config::app_config::llm_token_profiles, error::AppError};
use a4n_domain::shooting_script::{ShootingSceneV1, ShootingShotV1};
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
            |output| validate_design_unit(output, unit),
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
    for fact in planned_unit
        .performance_facts
        .iter()
        .filter(|fact| fact.required)
    {
        if !required_beats
            .iter()
            .any(|beat| source_sets_overlap(&beat.source_span_ids, &fact.source_span_ids))
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
    if shot.source_span_ids.is_empty()
        || shot
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
        if beat.source_span_ids.is_empty()
            || beat
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

fn source_sets_overlap(left: &[String], right: &[String]) -> bool {
    left.iter().any(|value| right.contains(value))
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

    use super::{optional_actor_is_allowed, shooting_design_call_order, source_sets_overlap};

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
        let fact_sources = vec!["span-1".to_string(), "span-2".to_string()];
        let beat_sources = vec![
            "span-1".to_string(),
            "span-2".to_string(),
            "span-3".to_string(),
        ];

        assert!(source_sets_overlap(&fact_sources, &beat_sources));
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
