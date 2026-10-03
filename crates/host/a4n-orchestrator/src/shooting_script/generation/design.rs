use std::collections::HashSet;

use a4n_domain::{
    chapter_visual_plan::VisualSegmentProjectionV3,
    shooting_script::{ShootingSceneV2, ShootingShotV2},
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

use super::super::{
    context::shooting_context_request,
    contracts::{
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT, CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        PreparedChapterShootingScriptRequest, ScreenplayPerformanceFactV1, ScreenplayPlanUnit,
        ShootingScriptProgressReporter, ShootingScriptStreamReporter,
    },
    progress::{design_shot_stream_progress, design_unit_stream_progress},
};

pub(super) async fn generate_shooting_scene(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    unit: &ScreenplayPlanUnit,
    unit_offset: usize,
    total_unit_count: usize,
    progress: ShootingScriptProgressReporter<'_>,
    stream_reporter: ShootingScriptStreamReporter<'_>,
    cancel_token: &CancellationToken,
) -> Result<super::super::contracts::ShootingScriptStageResult<ShootingSceneV2>, AppError> {
    let planned_unit = prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?;
    let planned_shots = planned_unit
        .shots
        .iter()
        .filter(|shot| prepared.shot_ids.is_empty() || prepared.shot_ids.contains(&shot.segment_id))
        .cloned()
        .collect::<Vec<_>>();
    let unit_progress = design_unit_stream_progress(unit_offset, total_unit_count);
    progress(
        unit_progress.span.start(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!("正在设计拍摄单元 {}/{}", unit_offset + 1, total_unit_count),
    )?;
    let mut scene = None;
    let mut shots = Vec::with_capacity(planned_unit.shots.len());
    let mut restored = true;
    let preceding_shot_count = prepared
        .visual_plan_projection
        .narrative_units
        .iter()
        .take(unit_offset)
        .try_fold(0usize, |count, unit| count.checked_add(unit.shots.len()))
        .ok_or_else(|| AppError::InvalidArgument("拍摄镜头数量超出 usize 范围".into()))?;
    for (shot_offset, planned_shot) in planned_shots.iter().enumerate() {
        let facts = unit
            .performance_facts
            .iter()
            .filter(|fact| fact.shot_id == planned_shot.segment_id)
            .collect::<Vec<_>>();
        let shot_progress = design_shot_stream_progress(
            unit_offset,
            total_unit_count,
            shot_offset,
            planned_shots.len(),
        );
        let step_key = format!(
            "shooting_design_shot:{}:{}",
            unit.narrative_unit_index, planned_shot.segment_id
        );
        let request = design_shot_request(prepared, workflow_run_id, unit, planned_shot, &facts)?;
        let stage = orchestrator
            .run_shooting_script_stage::<ShootingSceneV2>(
                workflow_run_id,
                &step_key,
                shooting_design_call_order(total_unit_count, preceding_shot_count, shot_offset)?,
                request,
                "拍摄设计镜头",
                shot_progress,
                CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
                progress,
                stream_reporter,
                cancel_token,
                Some(&prepared.source_markers),
                |output| validate_design_shot(output, unit, planned_shot, &facts, prepared),
            )
            .await?;
        restored &= stage.restored;
        let mut output = stage.output;
        shots.append(&mut output.shots);
        scene.get_or_insert(output);
        progress(
            shot_progress.span.end(),
            CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
            &format!(
                "拍摄单元 {}/{} 的镜头 {}/{} {}，已校验并保存断点",
                unit_offset + 1,
                total_unit_count,
                shot_offset + 1,
                planned_shots.len(),
                if stage.restored {
                    "已从断点恢复"
                } else {
                    "已生成"
                }
            ),
        )?;
    }
    let mut scene = scene.ok_or_else(|| AppError::InvalidArgument("拍摄设计单元无镜头".into()))?;
    scene.shots = shots;
    validate_design_unit(&scene, unit, prepared, &planned_shots)?;
    scene = super::super::assembly::assemble_scene(
        unit_offset,
        scene,
        unit,
        planned_unit,
        &planned_shots,
    )?;
    progress(
        unit_progress.span.end(),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        &format!(
            "拍摄单元 {}/{} {}并通过校验",
            unit_offset + 1,
            total_unit_count,
            if restored {
                "已从断点恢复"
            } else {
                "已生成"
            }
        ),
    )?;
    Ok(super::super::contracts::ShootingScriptStageResult {
        output: scene,
        restored,
    })
}

fn design_shot_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    unit: &ScreenplayPlanUnit,
    planned_shot: &VisualSegmentProjectionV3,
    facts: &[&ScreenplayPerformanceFactV1],
) -> Result<ChatRequest, AppError> {
    let planned_unit = prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?;
    let mut unit_context = serde_json::to_value(unit)?;
    unit_context
        .as_object_mut()
        .expect("screenplay unit")
        .remove("performance_facts");
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SHOOTING_DESIGN_UNIT_PROMPT,
        format!(
            "shooting_design_shot:{}:{}",
            unit.narrative_unit_index, planned_shot.segment_id
        ),
        prepared.design_unit_system_prompt.clone(),
        (&unit.source_span_ids, &planned_unit.scene_binding),
        Some((
            "shooting_visual_design_input",
            &serde_json::json!({
                "screenplay_plan_unit": unit_context,
                "locked_visual_unit": super::super::context::visual_unit_prompt_context(planned_unit)?,
                "target_shot": planned_shot,
                "required_performance_facts": facts,
            }),
        )),
        format!(
            "只设计 narrative_unit_index={} 的 target_shot_id={}。响应仍为 ShootingSceneV2，但 shots 必须只包含 target_shot_id 对应的一个镜头；不得输出其他镜头。不得改变镜头段 ID、时间范围、图片关键帧或场景绑定。required_performance_facts 中的 required fact 必须由本镜头的 required performance beat 覆盖，并逐字保留对应 source_span_ids 或 user_addition_ids。source_span_ids 只填写实际引用的正文片段；仅来源于用户导演描述的内容允许为空。",
            unit.narrative_unit_index, planned_shot.segment_id,
        ),
    )
}

fn validate_design_shot(
    scene: &ShootingSceneV2,
    unit: &ScreenplayPlanUnit,
    planned_shot: &VisualSegmentProjectionV3,
    facts: &[&ScreenplayPerformanceFactV1],
    prepared: &PreparedChapterShootingScriptRequest,
) -> Result<(), AppError> {
    if scene.narrative_unit_index != unit.narrative_unit_index || scene.shots.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计 unit {} 必须只生成一个目标镜头",
            unit.narrative_unit_index
        )));
    }
    let shot = &scene.shots[0];
    if shot.shot_id != planned_shot.segment_id {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计镜头 ID 偏离导演编排：期望 {}，实际 {}",
            planned_shot.segment_id, shot.shot_id
        )));
    }
    let allowed_sources = unit
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let participants = unit
        .participants
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    validate_scene_actors(scene, &participants)?;
    validate_shot_sources(
        scene.narrative_unit_index,
        shot,
        &allowed_sources,
        &participants,
    )?;
    validate_required_fact_coverage(scene, facts.iter().copied())?;
    super::screenplay::validate_transition_contracts(
        &scene.transition_contracts,
        unit.narrative_unit_index,
        prepared.visual_plan_projection.narrative_units.len(),
    )?;
    // 当前调用只装配目标镜头，内容校验必须先于调用断点和下一镜头请求。
    let planned = prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?;
    let assembled = super::super::assembly::assemble_scene(
        (unit.narrative_unit_index - 1) as usize,
        scene.clone(),
        unit,
        planned,
        std::slice::from_ref(planned_shot),
    )?;
    assembled
        .validate_content(&prepared.visual_plan_projection.user_additions, true)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

fn validate_design_unit(
    scene: &ShootingSceneV2,
    unit: &ScreenplayPlanUnit,
    prepared: &PreparedChapterShootingScriptRequest,
    planned_shots: &[VisualSegmentProjectionV3],
) -> Result<(), AppError> {
    if scene.narrative_unit_index != unit.narrative_unit_index || scene.shots.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元 {} 的身份无效或镜头为空",
            unit.narrative_unit_index
        )));
    }
    let planned = prepared.visual_unit(unit.narrative_unit_index, &unit.scene_identity)?;
    if scene.shots.len() != planned_shots.len() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计 unit {} 必须生成 {} 个视觉方案镜头，实际 {} 个",
            unit.narrative_unit_index,
            planned_shots.len(),
            scene.shots.len()
        )));
    }
    for (shot, expected) in scene.shots.iter().zip(planned_shots) {
        if shot.shot_id != expected.segment_id {
            return Err(AppError::InvalidArgument(format!(
                "拍摄设计镜头 ID 或顺序偏离导演编排：期望 {}，实际 {}",
                expected.segment_id, shot.shot_id
            )));
        }
        if shot
            .source_span_ids
            .iter()
            .any(|id| !planned.source_span_ids.contains(id))
        {
            return Err(AppError::InvalidArgument(format!(
                "拍摄设计 unit {} 的镜头来源顺序偏离 confirmed 导演编排",
                unit.narrative_unit_index
            )));
        }
    }
    Ok(())
}

fn validate_scene_actors(
    scene: &ShootingSceneV2,
    participants: &HashSet<&str>,
) -> Result<(), AppError> {
    if !optional_actor_is_allowed(scene.focal_character.as_deref(), participants) {
        return Err(AppError::InvalidArgument(format!(
            "镜头设计 unit {} 的 focal_character 不属于 participants",
            scene.narrative_unit_index
        )));
    }
    if !optional_actor_is_allowed(scene.viewpoint.anchor_character.as_deref(), participants) {
        return Err(AppError::InvalidArgument(format!(
            "镜头设计 unit {} 的 viewpoint.anchor_character 不属于 participants",
            scene.narrative_unit_index
        )));
    }
    Ok(())
}

fn validate_required_fact_coverage<'a>(
    scene: &ShootingSceneV2,
    facts: impl IntoIterator<Item = &'a ScreenplayPerformanceFactV1>,
) -> Result<(), AppError> {
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
    let required_beat_additions = required_source_ids(
        required_beats
            .iter()
            .map(|beat| beat.user_addition_ids.as_slice()),
    );
    for fact in facts.into_iter().filter(|fact| fact.required) {
        if !fact
            .source_span_ids
            .iter()
            .any(|id| required_beat_sources.contains(id.as_str()))
            && !fact
                .user_addition_ids
                .iter()
                .any(|id| required_beat_additions.contains(id.as_str()))
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
    shot: &ShootingShotV2,
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
        let addition_id = crate::chapter_visual_plan::segment_addition_id(&shot.shot_id);
        if beat.user_addition_ids.iter().any(|id| id != &addition_id) {
            return Err(AppError::InvalidArgument(format!(
                "镜头 {} 的表演节拍引用了其他镜头的导演事实",
                shot.shot_id
            )));
        }
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

fn shooting_design_call_order(
    total_unit_count: usize,
    preceding_shot_count: usize,
    shot_offset: usize,
) -> Result<i32, AppError> {
    let order = total_unit_count
        .checked_mul(3)
        .and_then(|value| value.checked_add(4))
        .and_then(|value| value.checked_add(preceding_shot_count))
        .and_then(|value| value.checked_add(shot_offset))
        .ok_or_else(|| AppError::InvalidArgument("拍摄镜头数量超出 usize 范围".into()))?;
    i32::try_from(order)
        .map_err(|_| AppError::InvalidArgument("拍摄设计单元数量超出 i32 范围".into()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{optional_actor_is_allowed, required_source_ids, shooting_design_call_order};

    #[test]
    fn design_call_order_starts_after_unit_stages_and_is_unique() {
        assert_eq!(
            shooting_design_call_order(2, 0, 0).expect("first design order"),
            10
        );
        assert_eq!(
            shooting_design_call_order(2, 1, 0).expect("second design order"),
            11
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
