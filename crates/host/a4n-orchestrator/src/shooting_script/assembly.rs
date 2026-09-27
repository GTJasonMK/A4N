use std::collections::{BTreeSet, HashMap};

use super::contracts::ScreenplayPlanUnit;
use crate::chapter_visual_plan::DirectorScriptUnit;
use a4n_domain::{
    chapter_visual_plan::VisualSegmentProjectionV3,
    shooting_script::{ChapterShootingScriptV2, ShootingSceneV2, ShootingShotV2},
};
use a4n_error::AppError;

pub(super) fn finish_generated_script(
    script: &mut ChapterShootingScriptV2,
    completed_unit_count: usize,
    total_unit_count: usize,
) -> Result<(), AppError> {
    let present_units = script
        .scenes
        .iter()
        .map(|scene| scene.narrative_unit_index)
        .collect::<BTreeSet<_>>();
    for scene in &mut script.scenes {
        // 已处理但不存在的单元由作者删除；未生成的目标仍保留其转场约定。
        scene.transition_contracts.retain(|transition| {
            transition.to_narrative_unit_index as usize > completed_unit_count
                || present_units.contains(&transition.to_narrative_unit_index)
        });
    }
    script
        .validate_partial(completed_unit_count, total_unit_count)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

pub(super) fn assemble_scene(
    index: usize,
    mut scene: ShootingSceneV2,
    unit: &ScreenplayPlanUnit,
    planned: &DirectorScriptUnit,
    planned_shots: &[VisualSegmentProjectionV3],
) -> Result<ShootingSceneV2, AppError> {
    if unit.scene_identity != planned.id || unit.sequence_index != planned.sequence_index {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元 {} 的顺序与 confirmed 导演编排不一致",
            unit.narrative_unit_index
        )));
    }
    if scene.shots.len() != planned_shots.len() {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计 unit {} 的镜头数量与导演编排不一致",
            unit.narrative_unit_index
        )));
    }
    scene.scene_index = checked_index(index)?;
    scene.scene_identity = planned.id.clone();
    scene.sequence_index = planned.sequence_index;
    scene.timeline_position = planned.timeline_position;
    scene.narrative_unit_index = unit.narrative_unit_index;
    scene.intercut_group_id = planned.intercut_group_id.clone();
    scene.editing_mode = planned.editing_mode;
    scene.visual_narrative_unit_id = Some(planned.id.clone());
    scene.user_addition_ids = planned_shots
        .iter()
        .map(|shot| format!("director:{}", shot.segment_id))
        .collect();
    scene.title = planned.title.clone();
    for (index, (shot, constraint)) in scene.shots.iter_mut().zip(planned_shots).enumerate() {
        apply_visual_shot_plan(shot, constraint, index)?;
    }
    // Source spans are actual model citations, not uniformly assigned timeline buckets.
    scene.source_span_ids = scene
        .shots
        .iter()
        .flat_map(|shot| shot.source_span_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(scene)
}

fn apply_visual_shot_plan(
    shot: &mut ShootingShotV2,
    planned: &VisualSegmentProjectionV3,
    index: usize,
) -> Result<(), AppError> {
    shot.shot_index = checked_index(index)?;
    shot.shot_id = planned.segment_id.clone();
    shot.title = planned.title.clone();
    shot.narrative_purpose = planned.prompt.clone();
    shot.user_addition_ids = vec![format!("director:{}", planned.segment_id)];
    let mut beat_ids = HashMap::new();
    let mut cited = shot
        .source_span_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for (index, beat) in shot.performance_beats.iter_mut().enumerate() {
        let original = beat.beat_id.clone();
        beat.beat_index = checked_index(index)?;
        beat.beat_id = format!("{}_beat_{}", shot.shot_id, beat.beat_index);
        cited.extend(beat.source_span_ids.iter().cloned());
        if beat_ids.insert(original, beat.beat_id.clone()).is_some() {
            return Err(AppError::InvalidArgument("模型返回了重复的 beat_id".into()));
        }
    }
    shot.source_span_ids = cited.into_iter().collect();
    for cue in &mut shot.dialogue_cues {
        cue.beat_id = beat_ids
            .get(&cue.beat_id)
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("对白引用了不存在的 beat_id".into()))?;
    }
    shot.bind_director_constraints(planned)
        .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

fn checked_index(index: usize) -> Result<u32, AppError> {
    u32::try_from(index + 1).map_err(|_| AppError::InvalidArgument("镜头数量超出 u32 范围".into()))
}
