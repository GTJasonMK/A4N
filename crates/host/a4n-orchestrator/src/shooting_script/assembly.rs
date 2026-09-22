use std::collections::{BTreeSet, HashMap};

use super::contracts::ScreenplayPlanUnit;
use crate::chapter_visual_plan::{DirectorScriptPlan, DirectorScriptUnit};
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;
use a4n_domain::{
    chapter_visual_plan::VisualSegmentProjectionV3,
    shooting_script::{
        ChapterShootingScriptSchemaVersion, ChapterShootingScriptV1, ShootingDirectingStatementV1,
        ShootingSceneV1, ShootingShotV1, ShootingVisualPlanSourceV1,
    },
};
use a4n_error::AppError;

pub(super) fn assemble_chapter_shooting_script(
    directing_statement: &ShootingDirectingStatementV1,
    units: &[ScreenplayPlanUnit],
    scenes: Vec<ShootingSceneV1>,
    visual_plan_record: &ParsedChapterVisualPlan,
    visual_plan: &DirectorScriptPlan,
) -> Result<ChapterShootingScriptV1, AppError> {
    let script = assemble_script(
        directing_statement,
        units,
        scenes,
        visual_plan_record,
        visual_plan,
        false,
    )?;
    script
        .validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    Ok(script)
}

pub(super) fn assemble_available_shooting_script(
    directing_statement: &ShootingDirectingStatementV1,
    units: &[ScreenplayPlanUnit],
    scenes: Vec<ShootingSceneV1>,
    visual_plan_record: &ParsedChapterVisualPlan,
    visual_plan: &DirectorScriptPlan,
) -> Result<ChapterShootingScriptV1, AppError> {
    assemble_script(
        directing_statement,
        units,
        scenes,
        visual_plan_record,
        visual_plan,
        true,
    )
}

fn assemble_script(
    directing_statement: &ShootingDirectingStatementV1,
    units: &[ScreenplayPlanUnit],
    scenes: Vec<ShootingSceneV1>,
    record: &ParsedChapterVisualPlan,
    plan: &DirectorScriptPlan,
    allow_partial: bool,
) -> Result<ChapterShootingScriptV1, AppError> {
    validate_unit_counts(
        scenes.len(),
        units.len(),
        plan.narrative_units.len(),
        allow_partial,
    )?;
    let mut scenes = scenes
        .into_iter()
        .zip(units)
        .enumerate()
        .map(|(index, (scene, unit))| {
            let planned = plan
                .narrative_units
                .iter()
                .find(|planned| planned.id == unit.scene_identity)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "拍摄设计单元 {} 找不到对应的导演编排",
                        unit.narrative_unit_index
                    ))
                })?;
            assemble_scene(index, scene, unit, planned)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let present_units = scenes
        .iter()
        .map(|scene| scene.narrative_unit_index)
        .collect::<BTreeSet<_>>();
    let highest_unit = present_units
        .iter()
        .next_back()
        .copied()
        .unwrap_or_default();
    for (index, scene) in scenes.iter_mut().enumerate() {
        if allow_partial {
            scene.timeline_position = checked_index(index)?;
        }
        scene.transition_contracts.retain(|transition| {
            transition.to_narrative_unit_index > highest_unit
                || present_units.contains(&transition.to_narrative_unit_index)
        });
    }
    Ok(ChapterShootingScriptV1 {
        schema_version: ChapterShootingScriptSchemaVersion::V1,
        visual_plan_source: Some(ShootingVisualPlanSourceV1 {
            plan_id: record.record.id,
            plan_version: record.record.plan_version,
            plan_sha256: record.record.plan_sha256.clone(),
        }),
        user_additions: plan.user_additions.clone(),
        directing_statement: Some(directing_statement.clone()),
        scenes,
    })
}

fn validate_unit_counts(
    scene_count: usize,
    unit_count: usize,
    planned_count: usize,
    allow_partial: bool,
) -> Result<(), AppError> {
    let valid = scene_count == unit_count
        && if allow_partial {
            unit_count <= planned_count
        } else {
            unit_count == planned_count
        };
    if valid {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "拍摄设计与 confirmed 导演编排的单元数量不一致：scenes={scene_count}, units={unit_count}, planned={planned_count}"
    )))
}

fn assemble_scene(
    index: usize,
    mut scene: ShootingSceneV1,
    unit: &ScreenplayPlanUnit,
    planned: &DirectorScriptUnit,
) -> Result<ShootingSceneV1, AppError> {
    if unit.scene_identity != planned.id || unit.sequence_index != planned.sequence_index {
        return Err(AppError::InvalidArgument(format!(
            "拍摄设计单元 {} 的顺序与 confirmed 导演编排不一致",
            unit.narrative_unit_index
        )));
    }
    if scene.shots.len() != planned.shots.len() {
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
    scene.intercut_group_id = None;
    scene.editing_mode = planned.editing_mode;
    scene.visual_narrative_unit_id = Some(planned.id.clone());
    scene.user_addition_ids = planned
        .shots
        .iter()
        .map(|shot| format!("director:{}", shot.segment_id))
        .collect();
    scene.title = planned.title.clone();
    for (index, (shot, constraint)) in scene.shots.iter_mut().zip(&planned.shots).enumerate() {
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
    shot: &mut ShootingShotV1,
    planned: &VisualSegmentProjectionV3,
    index: usize,
) -> Result<(), AppError> {
    shot.shot_index = checked_index(index)?;
    shot.shot_id = planned.segment_id.clone();
    shot.visual_shot_id = Some(planned.segment_id.clone());
    shot.planned_start_ms = Some(planned.start_ms);
    shot.planned_end_ms = Some(planned.end_ms);
    shot.title = planned.title.clone();
    shot.user_addition_ids = vec![format!("director:{}", planned.segment_id)];
    shot.visual_constraints = Some(planned.clone());
    shot.narrative_purpose = planned.prompt.clone();
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
    let duration = (planned.end_ms - planned.start_ms) as f64 / 1000.0;
    let seconds = shot
        .performance_beats
        .iter()
        .map(|beat| beat.estimated_seconds)
        .sum::<f64>();
    if !shot.performance_beats.is_empty() {
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(AppError::InvalidArgument(
                "表演节拍时长必须为有效正数".into(),
            ));
        }
        for beat in &mut shot.performance_beats {
            beat.estimated_seconds *= duration / seconds;
        }
    }
    shot.estimated_natural_seconds = duration;
    Ok(())
}

fn checked_index(index: usize) -> Result<u32, AppError> {
    u32::try_from(index + 1).map_err(|_| AppError::InvalidArgument("镜头数量超出 u32 范围".into()))
}

#[cfg(test)]
mod tests {
    use super::validate_unit_counts;

    #[test]
    fn available_script_accepts_a_confirmed_plan_prefix() {
        assert!(validate_unit_counts(2, 2, 5, true).is_ok());
        assert!(validate_unit_counts(2, 2, 5, false).is_err());
        assert!(validate_unit_counts(2, 3, 5, true).is_err());
        assert!(validate_unit_counts(2, 6, 5, true).is_err());
    }
}
