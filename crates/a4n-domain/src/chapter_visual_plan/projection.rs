use super::{
    ChapterVisualPlanV2, VisualCameraEventV2, VisualEntityEventV2, VisualKeyframeV2, VisualSceneV2,
    VisualShotV2, VisualSoundEventV2, VisualTechniqueEventV2,
};

#[derive(Debug, Clone, PartialEq)]
pub struct VisualPlanV2Projection {
    pub duration_ms: u64,
    pub scenes: Vec<VisualSceneProjectionV2>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualSceneProjectionV2 {
    pub scene: VisualSceneV2,
    pub sequence_index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub shots: Vec<VisualShotProjectionV2>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualShotProjectionV2 {
    pub shot: VisualShotV2,
    pub start_ms: u64,
    pub end_ms: u64,
    pub keyframes: Vec<VisualKeyframeProjectionV2>,
    pub entity_events: Vec<VisualEntityEventProjectionV2>,
    pub camera_events: Vec<VisualCameraEventProjectionV2>,
    pub sound_events: Vec<VisualSoundEventProjectionV2>,
    pub technique_events: Vec<VisualTechniqueEventProjectionV2>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualKeyframeProjectionV2 {
    pub keyframe: VisualKeyframeV2,
    pub at_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualEntityEventProjectionV2 {
    pub event: VisualEntityEventV2,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualCameraEventProjectionV2 {
    pub event: VisualCameraEventV2,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualSoundEventProjectionV2 {
    pub event: VisualSoundEventV2,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualTechniqueEventProjectionV2 {
    pub event: VisualTechniqueEventV2,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl ChapterVisualPlanV2 {
    pub fn project(
        &self,
    ) -> Result<VisualPlanV2Projection, super::ChapterVisualPlanValidationError> {
        self.validate_confirmed()?;
        let mut scene_start_ms = 0_u64;
        let mut scenes = Vec::with_capacity(self.scenes.len());
        for (scene_offset, scene) in self.scenes.iter().enumerate() {
            let mut shot_start_ms = scene_start_ms;
            let mut shots = Vec::with_capacity(scene.shots.len());
            for shot in &scene.shots {
                let end_ms = shot_start_ms + shot.duration_ms;
                shots.push(VisualShotProjectionV2 {
                    shot: shot.clone(),
                    start_ms: shot_start_ms,
                    end_ms,
                    keyframes: shot
                        .keyframes
                        .iter()
                        .cloned()
                        .map(|keyframe| VisualKeyframeProjectionV2 {
                            at_ms: shot_start_ms + keyframe.at_ms,
                            keyframe,
                        })
                        .collect(),
                    entity_events: shot
                        .entity_events
                        .iter()
                        .cloned()
                        .map(|event| VisualEntityEventProjectionV2 {
                            start_ms: shot_start_ms + event.start_ms,
                            end_ms: shot_start_ms + event.end_ms,
                            event,
                        })
                        .collect(),
                    camera_events: shot
                        .camera_events
                        .iter()
                        .cloned()
                        .map(|event| VisualCameraEventProjectionV2 {
                            start_ms: shot_start_ms + event.start_ms,
                            end_ms: shot_start_ms + event.end_ms,
                            event,
                        })
                        .collect(),
                    sound_events: shot
                        .sound_events
                        .iter()
                        .cloned()
                        .map(|event| VisualSoundEventProjectionV2 {
                            start_ms: shot_start_ms + event.start_ms,
                            end_ms: shot_start_ms + event.end_ms,
                            event,
                        })
                        .collect(),
                    technique_events: shot
                        .technique_events
                        .iter()
                        .cloned()
                        .map(|event| VisualTechniqueEventProjectionV2 {
                            start_ms: shot_start_ms + event.start_ms,
                            end_ms: shot_start_ms + event.end_ms,
                            event,
                        })
                        .collect(),
                });
                shot_start_ms = end_ms;
            }
            let scene_end_ms = shot_start_ms;
            scenes.push(VisualSceneProjectionV2 {
                scene: scene.clone(),
                sequence_index: u32::try_from(scene_offset + 1).map_err(|_| {
                    super::ChapterVisualPlanValidationError {
                        path: "scenes".into(),
                        message: "场景数量超出 u32 范围".into(),
                    }
                })?,
                start_ms: scene_start_ms,
                end_ms: scene_end_ms,
                shots,
            });
            scene_start_ms = scene_end_ms;
        }
        Ok(VisualPlanV2Projection {
            duration_ms: scene_start_ms,
            scenes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_visual_plan::{
        ChapterVisualPlanSchemaVersion, VisualCameraStateV1, VisualFrameV2, VisualInterpolationV1,
        VisualKeyframeV2, VisualSceneV2, VisualShotV2,
    };
    use crate::shooting_script::{ShootingCoverageV1, ShootingEditingModeV1};

    fn shot(id: &str, duration_ms: u64) -> VisualShotV2 {
        let keyframe = |suffix: &str, at_ms| VisualKeyframeV2 {
            id: format!("{id}-{suffix}"),
            at_ms,
            label: suffix.into(),
            interpolation_to_next: VisualInterpolationV1::Hold,
            camera: VisualCameraStateV1 {
                center_x: 0.5,
                center_y: 0.5,
                zoom: 1.0,
                rotation_degrees: 0.0,
                focus_entity_id: None,
                framing_note: None,
            },
            entity_states: vec![],
            relations: vec![],
        };
        VisualShotV2 {
            id: id.into(),
            title: id.into(),
            duration_ms,
            narrative_purpose: "推进".into(),
            coverage: ShootingCoverageV1::MustShow,
            shot_size: None,
            camera_angle: None,
            camera_height: None,
            lens_intent: None,
            composition: None,
            keyframes: vec![keyframe("start", 0), keyframe("end", duration_ms)],
            entity_events: vec![],
            camera_events: vec![],
            sound_events: vec![],
            technique_events: vec![],
            continuity_notes: vec![],
            transition_to_next: None,
        }
    }

    #[test]
    fn projection_accumulates_scene_and_shot_relative_time() {
        let plan = ChapterVisualPlanV2 {
            schema_version: ChapterVisualPlanSchemaVersion::V2,
            frame: VisualFrameV2 {
                width: 1920,
                height: 1080,
                background: None,
            },
            entities: vec![],
            scenes: vec![
                VisualSceneV2 {
                    id: "scene-1".into(),
                    title: "一".into(),
                    narrative_purpose: "一".into(),
                    editing_mode: ShootingEditingModeV1::Continuity,
                    intercut_group_id: None,
                    location: None,
                    time_of_day: None,
                    participant_entity_ids: vec![],
                    focal_entity_id: None,
                    shots: vec![shot("shot-1", 1_000), shot("shot-2", 2_000)],
                },
                VisualSceneV2 {
                    id: "scene-2".into(),
                    title: "二".into(),
                    narrative_purpose: "二".into(),
                    editing_mode: ShootingEditingModeV1::Continuity,
                    intercut_group_id: None,
                    location: None,
                    time_of_day: None,
                    participant_entity_ids: vec![],
                    focal_entity_id: None,
                    shots: vec![shot("shot-3", 500)],
                },
            ],
        };
        let projection = plan.project().expect("projection");
        assert_eq!(projection.duration_ms, 3_500);
        assert_eq!(projection.scenes[0].shots[1].start_ms, 1_000);
        assert_eq!(projection.scenes[1].start_ms, 3_000);
        assert_eq!(projection.scenes[1].end_ms, 3_500);
    }
}
