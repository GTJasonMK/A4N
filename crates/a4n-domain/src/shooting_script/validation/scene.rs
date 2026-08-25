use std::collections::HashSet;

use super::ShootingScriptValidationError;
use super::rules::{invalid, is_subset, non_empty_texts, require_index, required_text};
use crate::shooting_script::{ShootingPerformanceBeatV1, ShootingSceneV1, ShootingShotV1};

pub(super) fn validate_scene(
    scene: &ShootingSceneV1,
    path: &str,
    shot_ids: &mut HashSet<String>,
    beat_ids: &mut HashSet<String>,
    user_addition_ids: &HashSet<&str>,
    requires_visual_plan: bool,
) -> Result<(), ShootingScriptValidationError> {
    for (field, text) in [
        ("title", &scene.title),
        ("slugline", &scene.slugline),
        ("dramatic_question", &scene.dramatic_question),
        ("scene_objective", &scene.scene_objective),
        ("scene_obstacle", &scene.scene_obstacle),
        ("relationship_before", &scene.relationship_before),
        ("relationship_after", &scene.relationship_after),
        ("sound_strategy", &scene.sound_strategy),
        ("entry_state", &scene.entry_state),
        ("exit_state", &scene.exit_state),
    ] {
        required_text(text, &format!("{path}.{field}"))?;
    }
    validate_provenance(
        &scene.source_span_ids,
        &scene.user_addition_ids,
        &format!("{path}.source_span_ids"),
    )?;
    validate_addition_refs(
        &scene.user_addition_ids,
        user_addition_ids,
        &format!("{path}.user_addition_ids"),
    )?;
    if requires_visual_plan
        && scene
            .visual_narrative_unit_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return invalid(
            format!("{path}.visual_narrative_unit_id"),
            "绑定 visual plan 的拍摄剧本必须保留叙事单元 ID",
        );
    }
    if !scene.participants.is_empty() {
        non_empty_texts(&scene.participants, &format!("{path}.participants"))?;
    }
    if let Some(focal) = scene.focal_character.as_deref()
        && !scene
            .participants
            .iter()
            .any(|participant| participant == focal)
    {
        return invalid(format!("{path}.focal_character"), "必须属于 participants");
    }
    if let Some(anchor) = scene.viewpoint.anchor_character.as_deref()
        && !scene
            .participants
            .iter()
            .any(|participant| participant == anchor)
    {
        return invalid(
            format!("{path}.viewpoint.anchor_character"),
            "必须属于 participants",
        );
    }
    required_text(
        &scene.viewpoint.information_strategy,
        &format!("{path}.viewpoint.information_strategy"),
    )?;
    if scene.shots.is_empty() {
        return invalid(format!("{path}.shots"), "至少包含一个镜头");
    }
    for (shot_offset, shot) in scene.shots.iter().enumerate() {
        let shot_path = format!("{path}.shots[{shot_offset}]");
        require_index(
            shot.shot_index,
            shot_offset,
            &format!("{shot_path}.shot_index"),
        )?;
        validate_shot(
            scene,
            shot,
            &shot_path,
            shot_ids,
            beat_ids,
            user_addition_ids,
            requires_visual_plan,
        )?;
    }
    Ok(())
}

fn validate_shot(
    scene: &ShootingSceneV1,
    shot: &ShootingShotV1,
    path: &str,
    shot_ids: &mut HashSet<String>,
    beat_ids: &mut HashSet<String>,
    user_addition_ids: &HashSet<&str>,
    requires_visual_plan: bool,
) -> Result<(), ShootingScriptValidationError> {
    for (field, text) in [
        ("shot_id", &shot.shot_id),
        ("title", &shot.title),
        ("narrative_purpose", &shot.narrative_purpose),
        ("subject", &shot.subject),
        ("lens_intent", &shot.lens_intent),
        ("composition", &shot.composition),
        ("blocking.start", &shot.blocking.start),
        ("blocking.movement", &shot.blocking.movement),
        ("blocking.end", &shot.blocking.end),
        ("camera_start_state", &shot.camera_start_state),
        ("camera_end_state", &shot.camera_end_state),
        ("start_state", &shot.start_state),
        ("end_state", &shot.end_state),
        ("transition_in", &shot.transition_in),
        ("transition_out", &shot.transition_out),
    ] {
        required_text(text, &format!("{path}.{field}"))?;
    }
    if !shot_ids.insert(shot.shot_id.clone()) {
        return invalid(format!("{path}.shot_id"), "必须在章节内唯一");
    }
    validate_provenance(
        &shot.source_span_ids,
        &shot.user_addition_ids,
        &format!("{path}.source_span_ids"),
    )?;
    validate_addition_refs(
        &shot.user_addition_ids,
        user_addition_ids,
        &format!("{path}.user_addition_ids"),
    )?;
    match (shot.planned_start_ms, shot.planned_end_ms) {
        (Some(start), Some(end)) if start < end => {}
        (None, None) if !requires_visual_plan => {}
        _ => {
            return invalid(
                format!("{path}.planned_start_ms"),
                "visual plan 镜头必须同时保留有效的计划起止时间",
            );
        }
    }
    if requires_visual_plan
        && (shot
            .visual_shot_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
            || shot.visual_constraints.is_none())
    {
        return invalid(
            format!("{path}.visual_shot_id"),
            "绑定 visual plan 的拍摄剧本必须保留镜头 ID 和视觉约束",
        );
    }
    if !is_subset(&shot.source_span_ids, &scene.source_span_ids) {
        return invalid(
            format!("{path}.source_span_ids"),
            "必须属于所在场景的来源范围",
        );
    }
    if !shot.estimated_natural_seconds.is_finite() || shot.estimated_natural_seconds <= 0.0 {
        return invalid(
            format!("{path}.estimated_natural_seconds"),
            "必须是大于 0 的有限数",
        );
    }
    let mut seconds = 0.0;
    for (beat_offset, beat) in shot.performance_beats.iter().enumerate() {
        let beat_path = format!("{path}.performance_beats[{beat_offset}]");
        require_index(
            beat.beat_index,
            beat_offset,
            &format!("{beat_path}.beat_index"),
        )?;
        validate_beat(scene, shot, beat, &beat_path, beat_ids, user_addition_ids)?;
        seconds += beat.estimated_seconds;
    }
    if !shot.performance_beats.is_empty() && (seconds - shot.estimated_natural_seconds).abs() > 0.01
    {
        return invalid(
            format!("{path}.estimated_natural_seconds"),
            "必须等于表演节拍时长之和",
        );
    }
    let beat_ids_for_shot = shot
        .performance_beats
        .iter()
        .map(|beat| beat.beat_id.as_str())
        .collect::<HashSet<_>>();
    for cue in &shot.dialogue_cues {
        required_text(&cue.beat_id, &format!("{path}.dialogue_cues[].beat_id"))?;
        if !beat_ids_for_shot.contains(cue.beat_id.as_str()) {
            return invalid(
                format!("{path}.dialogue_cues[].beat_id"),
                "必须引用当前镜头的表演节拍",
            );
        }
        required_text(&cue.speaker, &format!("{path}.dialogue_cues[].speaker"))?;
        required_text(
            &cue.line_or_intent,
            &format!("{path}.dialogue_cues[].line_or_intent"),
        )?;
    }
    Ok(())
}

fn validate_beat(
    scene: &ShootingSceneV1,
    shot: &ShootingShotV1,
    beat: &ShootingPerformanceBeatV1,
    path: &str,
    beat_ids: &mut HashSet<String>,
    user_addition_ids: &HashSet<&str>,
) -> Result<(), ShootingScriptValidationError> {
    for (field, text) in [
        ("beat_id", &beat.beat_id),
        ("objective", &beat.objective),
        ("obstacle", &beat.obstacle),
        ("visible_action", &beat.visible_action),
        ("audience_question", &beat.audience_question),
        ("answer_or_delay", &beat.answer_or_delay),
        ("entry_state", &beat.entry_state),
        ("exit_state", &beat.exit_state),
    ] {
        required_text(text, &format!("{path}.{field}"))?;
    }
    if !beat_ids.insert(beat.beat_id.clone()) {
        return invalid(format!("{path}.beat_id"), "必须在章节内唯一");
    }
    validate_provenance(
        &beat.source_span_ids,
        &beat.user_addition_ids,
        &format!("{path}.source_span_ids"),
    )?;
    validate_addition_refs(
        &beat.user_addition_ids,
        user_addition_ids,
        &format!("{path}.user_addition_ids"),
    )?;
    if !is_subset(&beat.source_span_ids, &shot.source_span_ids) {
        return invalid(
            format!("{path}.source_span_ids"),
            "必须属于所在镜头的来源范围",
        );
    }
    if let Some(actor) = beat.actor.as_deref()
        && !scene
            .participants
            .iter()
            .any(|participant| participant == actor)
    {
        return invalid(format!("{path}.actor"), "必须属于所在场景 participants");
    }
    if !beat.estimated_seconds.is_finite() || beat.estimated_seconds <= 0.0 {
        return invalid(format!("{path}.estimated_seconds"), "必须是大于 0 的有限数");
    }
    Ok(())
}

fn validate_addition_refs(
    values: &[String],
    known: &HashSet<&str>,
    path: &str,
) -> Result<(), ShootingScriptValidationError> {
    if let Some(value) = values.iter().find(|value| !known.contains(value.as_str())) {
        invalid(path, format!("引用了不存在的用户新增事实: {value}"))
    } else {
        Ok(())
    }
}

fn validate_provenance(
    source_span_ids: &[String],
    user_addition_ids: &[String],
    path: &str,
) -> Result<(), ShootingScriptValidationError> {
    if source_span_ids.is_empty() && user_addition_ids.is_empty() {
        return invalid(path, "必须引用原文来源或用户新增事实");
    }
    if !source_span_ids.is_empty() {
        non_empty_texts(source_span_ids, path)?;
    }
    Ok(())
}
