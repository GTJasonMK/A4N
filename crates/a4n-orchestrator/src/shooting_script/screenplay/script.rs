use std::collections::{HashMap, HashSet};

use a4n_core::{
    error::AppError, shooting_script::ChapterShootingScriptV1, source_spans::SourceSpan,
};

use super::super::contracts::ChapterScreenplayPlan;
use super::super::contracts::ScreenplayPlanUnit;

pub(in crate::shooting_script) fn validate_script_source_spans(
    script: &ChapterShootingScriptV1,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    script
        .validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let allowed = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<HashSet<_>>();
    for scene in &script.scenes {
        for source_span_id in &scene.source_span_ids {
            if !allowed.contains(source_span_id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "章节共享拍摄剧本 scenes[{}] 引用了输入外 source_span_id: {}",
                    scene.scene_index, source_span_id
                )));
            }
        }
    }
    Ok(())
}

pub(in crate::shooting_script) fn validate_script_against_plan(
    script: &ChapterShootingScriptV1,
    plan: &ChapterScreenplayPlan,
) -> Result<(), AppError> {
    if script.scenes.len() != plan.units.len() {
        return Err(AppError::InvalidArgument(format!(
            "镜头设计单元数 {} 与剧本计划单元数 {} 不一致",
            script.scenes.len(),
            plan.units.len()
        )));
    }
    let planned_by_unit = plan
        .units
        .iter()
        .map(|scene| (scene.narrative_unit_index, scene))
        .collect::<HashMap<_, _>>();
    for scene in &script.scenes {
        let planned_unit = planned_by_unit
            .get(&scene.narrative_unit_index)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "镜头设计包含编排中不存在的 narrative unit {}",
                    scene.narrative_unit_index
                ))
            })?;
        if scene.scene_identity != planned_unit.scene_identity
            || scene.sequence_index != planned_unit.sequence_index
            || scene.timeline_position != planned_unit.timeline_position
            || scene.intercut_group_id != planned_unit.intercut_group_id
            || scene.editing_mode != planned_unit.editing_mode
            || scene.transition_contracts != planned_unit.transition_contracts
        {
            return Err(AppError::InvalidArgument(format!(
                "镜头设计 unit {} 的叙事编排身份与剧本计划不一致",
                scene.narrative_unit_index
            )));
        }
        validate_design_scene_content_against_plan_unit(scene, planned_unit)?;
    }
    Ok(())
}

pub(in crate::shooting_script) fn validate_design_scene_content_against_plan_unit(
    scene: &a4n_core::shooting_script::ShootingSceneV1,
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
        if shot.source_span_ids.is_empty()
            || shot
                .source_span_ids
                .iter()
                .any(|id| !allowed_sources.contains(id.as_str()))
        {
            return Err(AppError::InvalidArgument(format!(
                "镜头设计 unit {} 的 shot {} 引用了当前单元范围外的 source span",
                scene.narrative_unit_index, shot.shot_index
            )));
        }
        let shot_sources = shot
            .source_span_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        for beat in &shot.performance_beats {
            if !optional_actor_is_allowed(beat.actor.as_deref(), &participants) {
                return Err(AppError::InvalidArgument(format!(
                    "镜头设计 unit {} 的 shot {} beat {} actor 不属于 participants",
                    scene.narrative_unit_index, shot.shot_index, beat.beat_index
                )));
            }
            if beat.source_span_ids.is_empty()
                || beat
                    .source_span_ids
                    .iter()
                    .any(|id| !shot_sources.contains(id.as_str()))
            {
                return Err(AppError::InvalidArgument(format!(
                    "镜头设计 unit {} 的 shot {} beat {} 来源范围无效",
                    scene.narrative_unit_index, shot.shot_index, beat.beat_index
                )));
            }
        }
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
        let covered = required_beats
            .iter()
            .any(|beat| source_sets_overlap(&beat.source_span_ids, &fact.source_span_ids));
        if !covered {
            return Err(AppError::InvalidArgument(format!(
                "镜头设计 unit {} 的 required performance fact {} 未被 required beat 覆盖",
                scene.narrative_unit_index, fact.fact_index
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{optional_actor_is_allowed, source_sets_overlap};

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
