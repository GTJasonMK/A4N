use a4n_error::AppError;
use std::collections::HashSet;

use a4n_domain::shooting_script::{
    ChapterShootingScriptV1, ShootingCoverageV1, ShootingShotV1, ShootingViewpointModeV1,
};
use a4n_video_contract::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_EXECUTION_PROJECTION_V2, VideoActionLoadV1,
    VideoDirectingAuditV2, VideoExecutionBeatV2, VideoExecutionProjectionV2,
    VideoExecutionQualityV2, VideoExecutionRiskLevelV1, VideoExecutionRiskV1,
    VideoExecutionSceneV2, VideoExecutionShotV2, VideoShootingScriptIdentityV1,
    VideoSourceSpanRangeV1,
};

pub(in crate::video) fn video_projection_from_shooting_script(
    script: &ChapterShootingScriptV1,
    identity: VideoShootingScriptIdentityV1,
) -> Result<VideoExecutionProjectionV2, AppError> {
    let mut scenes = Vec::with_capacity(script.scenes.len());
    let mut total_seconds = 0.0;
    let mut total_beats = 0i32;
    let mut must_show_beats = 0i32;
    let mut total_recommended_segments = 0i32;
    let mut risk_flags = Vec::new();
    let mut seen_shot_ids = HashSet::new();
    let mut previous_timeline_position = None;
    let mut directing_order_preserved = true;
    let mut viewpoint_metadata_complete = true;
    let mut audience_causality_complete = true;

    for scene in &script.scenes {
        if previous_timeline_position.is_some_and(|position| scene.timeline_position <= position) {
            directing_order_preserved = false;
        }
        previous_timeline_position = Some(scene.timeline_position);
        viewpoint_metadata_complete &= scene_viewpoint_is_complete(scene);
        audience_causality_complete &= scene_audience_causality_is_complete(scene);

        let mut shots = Vec::with_capacity(scene.shots.len());
        let mut previous_shot_index = None;
        for shot in &scene.shots {
            let segment_duration_secs = shot
                .visual_constraints
                .as_ref()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("拍摄镜头 {} 未绑定导演编排", shot.shot_id))
                })?
                .video_duration_secs()
                .map_err(AppError::InvalidArgument)?;
            if previous_shot_index.is_some_and(|index| shot.shot_index <= index)
                || !seen_shot_ids.insert(shot.shot_id.clone())
            {
                directing_order_preserved = false;
            }
            previous_shot_index = Some(shot.shot_index);

            let action_beats = shot
                .performance_beats
                .iter()
                .map(|beat| {
                    total_beats = checked_increment(total_beats, "拍摄剧本表演节拍数量")?;
                    let required = beat.required || shot.coverage == ShootingCoverageV1::MustShow;
                    if required {
                        must_show_beats =
                            checked_increment(must_show_beats, "拍摄剧本必拍表演节拍数量")?;
                    }
                    audience_causality_complete &= !beat.audience_question.trim().is_empty()
                        && !beat.answer_or_delay.trim().is_empty();
                    Ok(VideoExecutionBeatV2 {
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
                        required,
                        natural_seconds: beat.estimated_seconds,
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?;

            total_recommended_segments =
                checked_increment(total_recommended_segments, "视频执行片段总数")?;
            total_seconds += shot.estimated_natural_seconds;
            let source_span_range = shot
                .source_span_ids
                .first()
                .zip(shot.source_span_ids.last())
                .map(|(start, end)| VideoSourceSpanRangeV1 {
                    start: start.clone(),
                    end: end.clone(),
                });
            let participants = shot_participants(shot);
            let focal_character = scene.focal_character.clone().filter(|name| participants.contains(name));
            shots.push(VideoExecutionShotV2 {
                director_constraints: shot.visual_constraints.clone(),
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
                estimated_natural_seconds: shot.estimated_natural_seconds,
                segment_duration_secs,
                recommended_segment_count: 1,
                minimum_readable_segment_count: 1,
                action_load: action_load(shot.estimated_natural_seconds),
                risk_flags: Vec::new(),
            });
        }

        scenes.push(VideoExecutionSceneV2 {
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
    append_audit_risk(
        &mut risk_flags,
        directing_order_preserved,
        "directing_order_not_preserved",
        "场景时间线、镜头顺序或稳定镜头 ID 不满足唯一有序约束",
    );
    append_audit_risk(
        &mut risk_flags,
        viewpoint_metadata_complete,
        "viewpoint_metadata_incomplete",
        "至少一个场景缺少可执行的视点锚点或信息策略",
    );
    append_audit_risk(
        &mut risk_flags,
        audience_causality_complete,
        "audience_causality_incomplete",
        "至少一个场景或表演节拍缺少观众问题、答案延迟或认知因果",
    );

    Ok(VideoExecutionProjectionV2 {
        schema_version: VIDEO_EXECUTION_PROJECTION_V2.to_string(),
        contract_epoch: VIDEO_EXECUTION_CONTRACT_EPOCH,
        shooting_script_identity: identity,
        directing_statement: script.directing_statement.clone(),
        scenes,
        planning_quality: VideoExecutionQualityV2 {
            total_estimated_seconds: total_seconds,
            total_recommended_segments,
            total_minimum_readable_segments: total_recommended_segments,
            risk_flags,
            directing_audit: VideoDirectingAuditV2 {
                total_directing_beats: total_beats,
                must_show_beats,
                directing_order_preserved,
                viewpoint_metadata_complete,
                audience_causality_complete,
            },
        },
    })
}

fn shot_participants(shot: &ShootingShotV1) -> Vec<String> {
    let mut seen = HashSet::new();
    shot.performance_beats.iter().filter_map(|beat| beat.actor.as_ref())
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
    shot.dialogue_cues.push(a4n_domain::shooting_script::ShootingDialogueCueV1 {
        beat_id: "voice".into(), speaker: "画外音".into(), line_or_intent: "停下".into(),
        listener_reaction: None,
    });
    assert_eq!(shot_participants(shot), ["画外音"]);
}

fn scene_viewpoint_is_complete(scene: &a4n_domain::shooting_script::ShootingSceneV1) -> bool {
    if scene.viewpoint.information_strategy.trim().is_empty() {
        return false;
    }
    match scene.viewpoint.mode {
        ShootingViewpointModeV1::OmniscientObjective => true,
        _ => scene
            .viewpoint
            .anchor_character
            .as_ref()
            .is_some_and(|anchor| scene.participants.contains(anchor)),
    }
}

fn scene_audience_causality_is_complete(
    scene: &a4n_domain::shooting_script::ShootingSceneV1,
) -> bool {
    !scene.dramatic_question.trim().is_empty()
        && !scene.scene_objective.trim().is_empty()
        && !scene.entry_state.trim().is_empty()
        && !scene.exit_state.trim().is_empty()
}

fn checked_increment(value: i32, field: &str) -> Result<i32, AppError> {
    value
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field}超出 i32 范围")))
}

fn action_load(seconds: f64) -> VideoActionLoadV1 {
    if seconds > 8.0 {
        VideoActionLoadV1::Heavy
    } else if seconds > 4.0 {
        VideoActionLoadV1::Medium
    } else {
        VideoActionLoadV1::Light
    }
}

fn append_audit_risk(
    risks: &mut Vec<VideoExecutionRiskV1>,
    passed: bool,
    code: &str,
    message: &str,
) {
    if !passed {
        risks.push(VideoExecutionRiskV1 {
            level: VideoExecutionRiskLevelV1::Error,
            code: code.to_string(),
            source_shot_id: None,
            message: message.to_string(),
        });
    }
}
