use a4n_error::AppError;
use std::collections::HashSet;

use a4n_domain::shooting_script::{ChapterShootingScriptV2, ShootingCoverageV1, ShootingShotV2};
use a4n_video_contract::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_EXECUTION_PROJECTION_V3, VideoExecutionBeatV3,
    VideoExecutionProjectionV3, VideoExecutionSceneV3, VideoExecutionShotV3,
    VideoShootingScriptIdentityV1, VideoSourceSpanRangeV1,
};

pub(in crate::video) fn video_projection_from_shooting_script(
    script: &ChapterShootingScriptV2,
    identity: VideoShootingScriptIdentityV1,
) -> Result<VideoExecutionProjectionV3, AppError> {
    let mut scenes = Vec::with_capacity(script.scenes.len());
    for scene in &script.scenes {
        let mut shots = Vec::with_capacity(scene.shots.len());
        for shot in &scene.shots {
            let director = shot.visual_constraints.as_ref().ok_or_else(|| {
                AppError::InvalidArgument(format!("拍摄镜头 {} 未绑定导演编排", shot.shot_id))
            })?;
            director
                .video_duration_secs()
                .map_err(AppError::InvalidArgument)?;
            let action_beats = shot
                .performance_beats
                .iter()
                .map(|beat| VideoExecutionBeatV3 {
                    beat_id: beat.beat_id.clone(),
                    source_span_ids: beat.source_span_ids.clone(),
                    actor: beat.actor.clone(),
                    objective: beat.objective.clone(),
                    obstacle: beat.obstacle.clone(),
                    visible_action: beat.visible_action.clone(),
                    reaction: beat.reaction.clone(),
                    expression: beat.expression.clone(),
                    gaze: beat.gaze.clone(),
                    pose_or_movement: beat.pose_or_movement.clone(),
                    dialogue_intent: beat.dialogue_intent.clone(),
                    audience_question: beat.audience_question.clone(),
                    answer_or_delay: beat.answer_or_delay.clone(),
                    knowledge_delta: beat.knowledge_delta.clone(),
                    relationship_delta: beat.relationship_delta.clone(),
                    spatial_change: beat.spatial_change.clone(),
                    entry_state: beat.entry_state.clone(),
                    exit_state: beat.exit_state.clone(),
                    required: beat.required || shot.coverage == ShootingCoverageV1::MustShow,
                })
                .collect();

            let source_span_range = shot
                .source_span_ids
                .first()
                .zip(shot.source_span_ids.last())
                .map(|(start, end)| VideoSourceSpanRangeV1 {
                    start: start.clone(),
                    end: end.clone(),
                });
            let participants = shot_participants(shot);
            let focal_character = scene
                .focal_character
                .clone()
                .filter(|name| participants.contains(name));
            shots.push(VideoExecutionShotV3 {
                director_constraints: director.clone(),
                source_shot_id: shot.shot_id.clone(),
                shot_index: shot.shot_index,
                scene_index: scene.scene_index,
                narrative_unit_index: scene.narrative_unit_index,
                timeline_position: scene.timeline_position,
                title: shot.title.clone(),
                source_span_ids: shot.source_span_ids.clone(),
                source_span_range,
                coverage: shot.coverage,
                narrative_purpose: shot.narrative_purpose.clone(),
                subject: shot.subject.clone(),
                participants,
                focal_character,
                viewpoint: scene.viewpoint.clone(),
                editing_mode: scene.editing_mode,
                intercut_group_id: scene.intercut_group_id.clone(),
                transition_contracts: scene.transition_contracts.clone(),
                audience_knowledge_before: scene.audience_knowledge_before.clone(),
                audience_knowledge_after: scene.audience_knowledge_after.clone(),
                withheld_information: scene.withheld_information.clone(),
                relationship_before: scene.relationship_before.clone(),
                relationship_after: scene.relationship_after.clone(),
                shot_size: shot.shot_size,
                camera_angle: shot.camera_angle,
                camera_height: shot.camera_height,
                lens_intent: shot.lens_intent.clone(),
                camera_movement: shot.camera_movement,
                composition: shot.composition.clone(),
                spatial_anchors: shot.spatial_anchors.clone(),
                blocking_start: shot.blocking.start.clone(),
                blocking_movement: shot.blocking.movement.clone(),
                blocking_end: shot.blocking.end.clone(),
                dialogue_cues: shot.dialogue_cues.clone(),
                sound: shot.sound.clone(),
                camera_start_state: shot.camera_start_state.clone(),
                camera_end_state: shot.camera_end_state.clone(),
                start_state: shot.start_state.clone(),
                end_state: shot.end_state.clone(),
                transition_in: shot.transition_in.clone(),
                transition_out: shot.transition_out.clone(),
                continuity_requirements: shot.continuity_requirements.clone(),
                action_beats,
            });
        }

        scenes.push(VideoExecutionSceneV3 {
            scene_index: scene.scene_index,
            scene_identity: scene.scene_identity.clone(),
            sequence_index: scene.sequence_index,
            narrative_unit_index: scene.narrative_unit_index,
            timeline_position: scene.timeline_position,
            intercut_group_id: scene.intercut_group_id.clone(),
            editing_mode: scene.editing_mode,
            transition_contracts: scene.transition_contracts.clone(),
            title: scene.title.clone(),
            slugline: scene.slugline.clone(),
            location: scene.location.clone(),
            time_of_day: scene.time_of_day.clone(),
            participants: scene.participants.clone(),
            focal_character: scene.focal_character.clone(),
            source_span_ids: scene.source_span_ids.clone(),
            dramatic_question: scene.dramatic_question.clone(),
            scene_objective: scene.scene_objective.clone(),
            scene_obstacle: scene.scene_obstacle.clone(),
            relationship_before: scene.relationship_before.clone(),
            relationship_after: scene.relationship_after.clone(),
            audience_knowledge_before: scene.audience_knowledge_before.clone(),
            audience_knowledge_after: scene.audience_knowledge_after.clone(),
            withheld_information: scene.withheld_information.clone(),
            viewpoint: scene.viewpoint.clone(),
            spatial_continuity: scene.spatial_continuity.clone(),
            sound_strategy: scene.sound_strategy.clone(),
            entry_state: scene.entry_state.clone(),
            exit_state: scene.exit_state.clone(),
            shots,
        });
    }

    if scenes.iter().all(|scene| scene.shots.is_empty()) {
        return Err(AppError::InvalidArgument(
            "结构化拍摄剧本没有可执行的视频镜头".to_string(),
        ));
    }
    Ok(VideoExecutionProjectionV3 {
        schema_version: VIDEO_EXECUTION_PROJECTION_V3.to_string(),
        contract_epoch: VIDEO_EXECUTION_CONTRACT_EPOCH,
        shooting_script_identity: identity,
        directing_statement: script.directing_statement.clone(),
        scenes,
    })
}

fn shot_participants(shot: &ShootingShotV2) -> Vec<String> {
    let mut seen = HashSet::new();
    shot.performance_beats
        .iter()
        .filter_map(|beat| beat.actor.as_ref())
        .chain(shot.dialogue_cues.iter().map(|cue| &cue.speaker))
        .filter(|name| seen.insert(*name))
        .cloned()
        .collect()
}

#[cfg(test)]
#[test]
fn shot_participants_use_performers_and_speakers_without_scene_bystanders() {
    let mut script = crate::video::tests::test_chapter_shooting_script("source");
    script.scenes[0].participants.push("在别处的人".into());
    let shot = &mut script.scenes[0].shots[0];
    assert_eq!(shot_participants(shot), ["小宦官"]);
    for beat in &mut shot.performance_beats {
        beat.actor = None;
    }
    assert!(shot_participants(shot).is_empty());
    shot.dialogue_cues
        .push(a4n_domain::shooting_script::ShootingDialogueCueV1 {
            beat_id: "voice".into(),
            speaker: "画外音".into(),
            line_or_intent: "停下".into(),
            listener_reaction: None,
        });
    assert_eq!(shot_participants(shot), ["画外音"]);
}
