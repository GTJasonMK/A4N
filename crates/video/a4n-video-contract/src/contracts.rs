use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use a4n_domain::shooting_script::{
    ShootingCameraAngleV1, ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingCoverageV1,
    ShootingDialogueCueV1, ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingShotSizeV1,
    ShootingSoundDesignV1, ShootingSpatialContinuityV1, ShootingTransitionContractV1,
    ShootingTransitionTypeV1, ShootingViewpointV1,
};

pub const VIDEO_EXECUTION_CONTRACT_EPOCH: u32 = 5;
pub const VIDEO_EXECUTION_PROJECTION_V3: &str = "video_execution_projection_v3";
pub const VIDEO_SEGMENT_PLAN_V2: &str = "video_segment_plan_v2";
pub const VIDEO_EXECUTION_UNIT_V2: &str = "video_execution_unit_v2";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoShootingScriptIdentityV1 {
    pub record_id: i64,
    pub revision: i32,
    pub source_revision_key: String,
    pub source_content_sha256: String,
    pub script_sha256: String,
    pub shot_fingerprints: std::collections::BTreeMap<String, String>,
}

impl VideoShootingScriptIdentityV1 {
    pub fn matches_shot(
        &self,
        source_content_sha256: &str,
        shot_id: &str,
        fingerprint: Option<&String>,
    ) -> bool {
        self.source_content_sha256 == source_content_sha256
            && fingerprint
                .is_some_and(|expected| self.shot_fingerprints.get(shot_id) == Some(expected))
    }

    /// 增量发布时，尚未更新的片段继续引用自己的旧镜头来源。
    pub fn carry_forward_sources(
        &mut self,
        previous: &Self,
        updated_shot_ids: &std::collections::HashSet<String>,
    ) -> Result<(), String> {
        if self.source_content_sha256 != previous.source_content_sha256
            && self
                .shot_fingerprints
                .keys()
                .any(|id| !updated_shot_ids.contains(id))
        {
            return Err("不能合并不同正文来源的片段规划".into());
        }
        self.shot_fingerprints.retain(|id, fingerprint| {
            if updated_shot_ids.contains(id) {
                return true;
            }
            if let Some(old) = previous.shot_fingerprints.get(id) {
                *fingerprint = old.clone();
                true
            } else {
                false
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod source_identity_tests {
    use super::VideoShootingScriptIdentityV1;

    #[test]
    fn source_revisions_do_not_invalidate_unchanged_shots_and_partial_updates_keep_old_sources() {
        let previous = VideoShootingScriptIdentityV1 {
            record_id: 1,
            revision: 1,
            source_revision_key: "source".into(),
            source_content_sha256: "a".repeat(64),
            script_sha256: "b".repeat(64),
            shot_fingerprints: [
                ("shot-1".into(), "old-1".into()),
                ("shot-2".into(), "old-2".into()),
            ]
            .into(),
        };
        let mut current = previous.clone();
        current.revision = 2;
        current.script_sha256 = "c".repeat(64);
        current
            .shot_fingerprints
            .insert("shot-2".into(), "new-2".into());
        let matches = |saved: &VideoShootingScriptIdentityV1, id| {
            saved.matches_shot(
                &current.source_content_sha256,
                id,
                current.shot_fingerprints.get(id),
            )
        };
        assert!(matches(&previous, "shot-1"));
        assert!(!matches(&previous, "shot-2"));
        let mut partial = current.clone();
        partial
            .carry_forward_sources(&previous, &["shot-1".into()].into())
            .unwrap();
        assert!(!matches(&partial, "shot-2"));
        assert!(matches(&partial, "shot-1"));
        let unknown = VideoShootingScriptIdentityV1 {
            shot_fingerprints: Default::default(),
            ..current.clone()
        };
        assert!(!matches(&unknown, "shot-1"));
        let mut missing_fingerprints = serde_json::to_value(&previous).unwrap();
        missing_fingerprints
            .as_object_mut()
            .unwrap()
            .remove("shot_fingerprints");
        assert!(
            serde_json::from_value::<VideoShootingScriptIdentityV1>(missing_fingerprints).is_err()
        );
        current.source_content_sha256 = "d".repeat(64);
        assert!(
            current
                .carry_forward_sources(&previous, &Default::default())
                .is_err()
        );
        assert!(
            current
                .carry_forward_sources(
                    &previous,
                    &current.shot_fingerprints.keys().cloned().collect()
                )
                .is_ok()
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionProjectionV3 {
    pub schema_version: String,
    pub contract_epoch: u32,
    pub shooting_script_identity: VideoShootingScriptIdentityV1,
    pub directing_statement: Option<ShootingDirectingStatementV1>,
    pub scenes: Vec<VideoExecutionSceneV3>,
}

impl VideoExecutionProjectionV3 {
    pub fn shots(&self) -> impl Iterator<Item = &VideoExecutionShotV3> {
        self.scenes.iter().flat_map(|scene| scene.shots.iter())
    }

    pub fn shot(&self, source_shot_id: &str) -> Option<&VideoExecutionShotV3> {
        self.shots()
            .find(|shot| shot.source_shot_id == source_shot_id)
    }

    pub fn shot_count(&self) -> usize {
        self.scenes.iter().map(|scene| scene.shots.len()).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionSceneV3 {
    pub scene_index: u32,
    pub scene_identity: String,
    pub sequence_index: u32,
    pub narrative_unit_index: u32,
    pub timeline_position: u32,
    pub intercut_group_id: Option<String>,
    pub editing_mode: ShootingEditingModeV1,
    pub transition_contracts: Vec<ShootingTransitionContractV1>,
    pub title: String,
    pub slugline: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants: Vec<String>,
    pub focal_character: Option<String>,
    pub source_span_ids: Vec<String>,
    pub dramatic_question: String,
    pub scene_objective: String,
    pub scene_obstacle: String,
    pub relationship_before: String,
    pub relationship_after: String,
    pub audience_knowledge_before: Vec<String>,
    pub audience_knowledge_after: Vec<String>,
    pub withheld_information: Vec<String>,
    pub viewpoint: ShootingViewpointV1,
    pub spatial_continuity: ShootingSpatialContinuityV1,
    pub sound_strategy: String,
    pub entry_state: String,
    pub exit_state: String,
    pub shots: Vec<VideoExecutionShotV3>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionShotV3 {
    pub director_constraints: a4n_domain::chapter_visual_plan::VisualSegmentProjectionV3,
    pub source_shot_id: String,
    pub shot_index: u32,
    pub scene_index: u32,
    pub narrative_unit_index: u32,
    pub timeline_position: u32,
    pub title: String,
    pub source_span_ids: Vec<String>,
    pub source_span_range: Option<VideoSourceSpanRangeV1>,
    pub coverage: ShootingCoverageV1,
    pub narrative_purpose: String,
    pub subject: String,
    pub participants: Vec<String>,
    pub focal_character: Option<String>,
    pub viewpoint: ShootingViewpointV1,
    pub editing_mode: ShootingEditingModeV1,
    pub intercut_group_id: Option<String>,
    pub transition_contracts: Vec<ShootingTransitionContractV1>,
    pub audience_knowledge_before: Vec<String>,
    pub audience_knowledge_after: Vec<String>,
    pub withheld_information: Vec<String>,
    pub relationship_before: String,
    pub relationship_after: String,
    pub shot_size: ShootingShotSizeV1,
    pub camera_angle: ShootingCameraAngleV1,
    pub camera_height: ShootingCameraHeightV1,
    pub lens_intent: String,
    pub camera_movement: ShootingCameraMovementV1,
    pub composition: String,
    pub spatial_anchors: Vec<String>,
    pub blocking_start: String,
    pub blocking_movement: String,
    pub blocking_end: String,
    pub dialogue_cues: Vec<ShootingDialogueCueV1>,
    pub sound: ShootingSoundDesignV1,
    pub camera_start_state: String,
    pub camera_end_state: String,
    pub start_state: String,
    pub end_state: String,
    pub transition_in: String,
    pub transition_out: String,
    pub continuity_requirements: Vec<String>,
    pub action_beats: Vec<VideoExecutionBeatV3>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionBeatV3 {
    pub beat_id: String,
    pub source_span_ids: Vec<String>,
    pub actor: Option<String>,
    pub objective: String,
    pub obstacle: String,
    pub visible_action: String,
    pub reaction: Option<String>,
    pub expression: Option<String>,
    pub gaze: Option<String>,
    pub pose_or_movement: Option<String>,
    pub dialogue_intent: Option<String>,
    pub audience_question: String,
    pub answer_or_delay: String,
    pub knowledge_delta: Vec<String>,
    pub relationship_delta: Option<String>,
    pub spatial_change: Option<String>,
    pub entry_state: String,
    pub exit_state: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSourceSpanRangeV1 {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentPlanV2 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub director_constraints: Option<a4n_domain::chapter_visual_plan::VisualSegmentProjectionV3>,
    pub schema_version: String,
    pub segment_number: i32,
    pub source_shot_id: String,
    pub segment_index_in_shot: i32,
    pub shot_segment_count: i32,
    pub covered_beat_ids: Vec<String>,
    pub title: String,
    pub scene_summary: String,
    pub blocking_summary: String,
    pub spatial_anchors: Vec<VideoSpatialAnchorV1>,
    pub emotion_action_goals: Vec<VideoEmotionActionGoalV1>,
    pub video_intent: VideoIntentV1,
    pub emotional_arc: VideoEmotionalArcV1,
    pub performance_plan: VideoPerformancePlanV2,
    pub material_coverage: VideoMaterialCoverageV1,
    pub transition_to_next: Option<VideoSegmentTransitionV1>,
    pub source_span_ids: Vec<String>,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSpatialAnchorV1 {
    pub anchor: String,
    pub usage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoEmotionActionGoalV1 {
    pub character: String,
    pub emotion: String,
    pub visible_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoIntentV1 {
    pub hook: String,
    pub audience_effect: String,
    pub rhythm: String,
    pub visual_priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoEmotionalArcV1 {
    pub start: String,
    pub turn: String,
    pub end: String,
    pub function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoPerformancePlanV2 {
    pub movement_path: String,
    pub body_performance: String,
    pub gaze_and_hands: String,
    pub pause_or_reaction: String,
    pub camera_relationship: String,
    pub time_blocks: Vec<VideoPerformanceTimeBlockV1>,
    pub action_script: Vec<VideoPerformanceActionV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoPerformanceTimeBlockV1 {
    pub start_sec: f64,
    pub end_sec: f64,
    pub camera: String,
    pub subject_action: String,
    pub continuity_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoPerformanceActionV1 {
    pub character: String,
    pub body: String,
    pub movement: String,
    pub hands: String,
    pub gaze: String,
    pub timing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoMaterialCoverageV1 {
    pub required_entities: Vec<String>,
    pub missing_or_weak_references: Vec<String>,
    pub risk_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentTransitionV1 {
    pub transition_type: ShootingTransitionTypeV1,
    pub editing_mode: ShootingEditingModeV1,
    pub preserved_action: Option<String>,
    pub preserved_sound: Option<String>,
    pub required_cut_point: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoExecutionUnitV2 {
    pub schema_version: String,
    pub unit_number: i32,
    pub duration_secs: i32,
    pub start_frame_number: Option<i32>,
    pub end_frame_number: Option<i32>,
    pub storyboard_number: Option<i32>,
    pub kind: VideoExecutionUnitKindV2,
    pub prompt_segment: VideoSegmentPlanV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "unit_type", rename_all = "snake_case")]
pub enum VideoExecutionUnitKindV2 {
    Content(VideoContentUnitV2),
    ActionBridge(VideoActionBridgeUnitV1),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoContentUnitV2 {
    pub source_segment_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoActionBridgeUnitV1 {
    pub from_segment_number: i32,
    pub to_segment_number: i32,
    pub preserved_action: String,
    pub preserved_sound: Option<String>,
    pub required_cut_point: Option<String>,
}

impl VideoExecutionUnitV2 {
    pub fn source_segment_numbers(&self) -> Vec<i32> {
        match &self.kind {
            VideoExecutionUnitKindV2::Content(content) => vec![content.source_segment_number],
            VideoExecutionUnitKindV2::ActionBridge(bridge) => {
                vec![bridge.from_segment_number, bridge.to_segment_number]
            }
        }
    }
}
