use std::collections::HashSet;

use a4n_core::{
    error::AppError,
    video_execution::{
        VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_EXECUTION_PROJECTION_V2, VideoActionLoadV1,
        VideoDirectingAuditV2, VideoExecutionBeatV2, VideoExecutionProjectionV2,
        VideoExecutionQualityV2, VideoExecutionRiskLevelV1, VideoExecutionRiskV1,
        VideoExecutionSceneV2, VideoExecutionShotV2, VideoShootingScriptIdentityV1,
        VideoSourceSpanRangeV1,
    },
};
use a4n_domain::shooting_script::{
    ChapterShootingScriptV1, ShootingCoverageV1, ShootingViewpointModeV1,
};

pub(in crate::video) fn video_projection_from_shooting_script(
    script: &ChapterShootingScriptV1,
    identity: VideoShootingScriptIdentityV1,
    segment_duration_secs: i32,
) -> Result<VideoExecutionProjectionV2, AppError> {
    if segment_duration_secs <= 0 {
        return Err(AppError::InvalidArgument(
            "视频执行片段时长必须大于 0".to_string(),
        ));
    }

    let mut scenes = Vec::with_capacity(script.scenes.len());
    let mut total_seconds = 0.0;
    let mut total_beats = 0i32;
    let mut must_show_beats = 0i32;
    let mut total_recommended_segments = 0i32;
    let mut total_minimum_readable_segments = 0i32;
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

            let recommended_segment_count = segment_count_for_seconds(
                shot.estimated_natural_seconds,
                segment_duration_secs,
                &shot.shot_id,
                "estimated_natural_seconds",
            )?;
            let required_seconds = action_beats
                .iter()
                .filter(|beat| beat.required)
                .map(|beat| beat.natural_seconds)
                .sum::<f64>();
            let minimum_readable_segment_count = if required_seconds > 0.0 {
                segment_count_for_seconds(
                    required_seconds,
                    segment_duration_secs,
                    &shot.shot_id,
                    "required beat seconds",
                )?
            } else {
                1
            };
            total_recommended_segments = checked_add(
                total_recommended_segments,
                recommended_segment_count,
                "视频执行建议片段总数",
            )?;
            total_minimum_readable_segments = checked_add(
                total_minimum_readable_segments,
                minimum_readable_segment_count,
                "视频执行最低可读片段总数",
            )?;
            total_seconds += shot.estimated_natural_seconds;

            let source_start = shot.source_span_ids.first().ok_or_else(|| {
                AppError::InvalidArgument(format!("拍摄镜头 {} 缺少 source_span_ids", shot.shot_id))
            })?;
            let source_end = shot.source_span_ids.last().ok_or_else(|| {
                AppError::InvalidArgument(format!("拍摄镜头 {} 缺少 source_span_ids", shot.shot_id))
            })?;
            let mut shot_risks = Vec::new();
            if minimum_readable_segment_count > recommended_segment_count {
                shot_risks.push(VideoExecutionRiskV1 {
                    level: VideoExecutionRiskLevelV1::Warning,
                    code: "required_beats_exceed_recommended_capacity".to_string(),
                    source_shot_id: Some(shot.shot_id.clone()),
                    message: "必拍节拍的自然耗时高于镜头总耗时推导出的建议容量".to_string(),
                });
            }
            risk_flags.extend(shot_risks.iter().cloned());
            shots.push(VideoExecutionShotV2 {
                source_shot_id: shot.shot_id.clone(),
                shot_index: shot.shot_index,
                scene_index: scene.scene_index,
                narrative_unit_index: scene.narrative_unit_index,
                timeline_position: scene.timeline_position,
                title: shot.title.clone(),
                source_span_ids: shot.source_span_ids.clone(),
                source_span_range: VideoSourceSpanRangeV1 {
                    start: source_start.clone(),
                    end: source_end.clone(),
                },
                coverage: shot.coverage,
                narrative_purpose: shot.narrative_purpose.clone(),
                subject: shot.subject.clone(),
                participants: scene.participants.clone(),
                focal_character: scene.focal_character.clone(),
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
                recommended_segment_count,
                minimum_readable_segment_count,
                action_load: action_load(shot.estimated_natural_seconds, recommended_segment_count),
                risk_flags: shot_risks,
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
            total_minimum_readable_segments,
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

fn segment_count_for_seconds(
    seconds: f64,
    duration_secs: i32,
    shot_id: &str,
    field: &str,
) -> Result<i32, AppError> {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(AppError::InvalidArgument(format!(
            "拍摄镜头 {shot_id} {field} 必须大于 0"
        )));
    }
    let count = (seconds / f64::from(duration_secs)).ceil().max(1.0);
    if count > f64::from(i32::MAX) {
        return Err(AppError::InvalidArgument(format!(
            "拍摄镜头 {shot_id} 所需视频执行片段数量超出 i32 范围"
        )));
    }
    Ok(count as i32)
}

fn checked_increment(value: i32, field: &str) -> Result<i32, AppError> {
    value
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field}超出 i32 范围")))
}

fn checked_add(value: i32, addition: i32, field: &str) -> Result<i32, AppError> {
    value
        .checked_add(addition)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field}超出 i32 范围")))
}

fn action_load(seconds: f64, segment_count: i32) -> VideoActionLoadV1 {
    if segment_count > 1 || seconds > 8.0 {
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
