use a4n_core::shooting_script::{
    ChapterShootingScriptV1, ShootingCoverageV1, ShootingPerformanceBeatV1,
};
use serde_json::{Value, json};

pub(super) fn manga_events_from_shooting_script(script: &ChapterShootingScriptV1) -> Value {
    Value::Array(
        script
            .scenes
            .iter()
            .flat_map(|scene| {
                scene.shots.iter().flat_map(move |shot| {
                    shot.performance_beats.iter().map(move |beat| {
                        let description = match beat.reaction.as_deref() {
                            Some(reaction) => format!("{}；反应：{reaction}", beat.visible_action),
                            None => beat.visible_action.clone(),
                        };
                        json!({
                            "beat_ref": {
                                "scene_index": scene.scene_index,
                                "shot_index": shot.shot_index,
                                "beat_index": beat.beat_index,
                            },
                            "type": event_type(beat),
                            "description": description,
                            "participants": beat.actor.iter().collect::<Vec<_>>(),
                            "importance": event_importance(shot.coverage, beat.required),
                            "is_climax": is_climax(beat),
                            "source_span_ids": beat.source_span_ids,
                        })
                    })
                })
            })
            .collect(),
    )
}

pub(super) fn manga_page_planning_source(
    shooting_script_id: i64,
    shooting_script_revision: i32,
    script: &ChapterShootingScriptV1,
) -> Value {
    json!({
        "shooting_script_id": shooting_script_id,
        "shooting_script_revision": shooting_script_revision,
        "directing_statement": script.directing_statement,
        "scene_continuity": script.scenes.iter().map(|scene| json!({
            "scene_index": scene.scene_index,
            "sequence_index": scene.sequence_index,
            "timeline_position": scene.timeline_position,
            "intercut_group_id": scene.intercut_group_id,
            "editing_mode": scene.editing_mode,
            "transition_contracts": scene.transition_contracts,
            "title": scene.title,
            "participants": scene.participants,
            "focal_character": scene.focal_character,
            "dramatic_question": scene.dramatic_question,
            "relationship_before": scene.relationship_before,
            "relationship_after": scene.relationship_after,
            "audience_knowledge_before": scene.audience_knowledge_before,
            "audience_knowledge_after": scene.audience_knowledge_after,
            "withheld_information": scene.withheld_information,
            "viewpoint": scene.viewpoint,
            "spatial_continuity": scene.spatial_continuity,
            "sound_strategy": scene.sound_strategy,
            "entry_state": scene.entry_state,
            "exit_state": scene.exit_state,
        })).collect::<Vec<_>>(),
        "events": manga_events_from_shooting_script(script),
    })
}

fn event_type(beat: &ShootingPerformanceBeatV1) -> &'static str {
    if beat.dialogue_intent.is_some() {
        "dialogue"
    } else if beat.reaction.is_some() {
        "reaction"
    } else if !beat.knowledge_delta.is_empty() {
        "revelation"
    } else if beat.spatial_change.is_some() {
        "transition"
    } else {
        "action"
    }
}

fn event_importance(coverage: ShootingCoverageV1, required: bool) -> &'static str {
    if required || coverage == ShootingCoverageV1::MustShow {
        "critical"
    } else if coverage == ShootingCoverageV1::ShouldShow {
        "high"
    } else {
        "normal"
    }
}

fn is_climax(beat: &ShootingPerformanceBeatV1) -> bool {
    beat.required && (!beat.knowledge_delta.is_empty() || beat.relationship_delta.is_some())
}
