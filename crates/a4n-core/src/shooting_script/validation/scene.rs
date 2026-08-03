use std::collections::BTreeSet;

use super::super::contracts::{ShootingPerformanceBeatV1, ShootingSceneV1, ShootingShotV1};
use super::ShootingScriptValidationError;
use super::rules::{invalid, is_subset, non_empty_texts, require_index, required_text};

pub(super) fn validate_scene(
    scene: &ShootingSceneV1,
    path: &str,
    shot_ids: &mut BTreeSet<String>,
    beat_ids: &mut BTreeSet<String>,
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
    non_empty_texts(&scene.source_span_ids, &format!("{path}.source_span_ids"))?;
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
        validate_shot(scene, shot, &shot_path, shot_ids, beat_ids)?;
    }
    Ok(())
}

fn validate_shot(
    scene: &ShootingSceneV1,
    shot: &ShootingShotV1,
    path: &str,
    shot_ids: &mut BTreeSet<String>,
    beat_ids: &mut BTreeSet<String>,
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
    non_empty_texts(&shot.source_span_ids, &format!("{path}.source_span_ids"))?;
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
        validate_beat(scene, shot, beat, &beat_path, beat_ids)?;
        seconds += beat.estimated_seconds;
    }
    if !shot.performance_beats.is_empty() && (seconds - shot.estimated_natural_seconds).abs() > 0.01
    {
        return invalid(
            format!("{path}.estimated_natural_seconds"),
            "必须等于表演节拍时长之和",
        );
    }
    for cue in &shot.dialogue_cues {
        required_text(&cue.beat_id, &format!("{path}.dialogue_cues[].beat_id"))?;
        if !shot
            .performance_beats
            .iter()
            .any(|beat| beat.beat_id == cue.beat_id)
        {
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
    beat_ids: &mut BTreeSet<String>,
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
    non_empty_texts(&beat.source_span_ids, &format!("{path}.source_span_ids"))?;
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
