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
