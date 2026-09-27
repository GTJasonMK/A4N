//! Frozen v1 wire layout. Only migrations, old packages and completed-call upgrades use it.
//! Field order is part of the historical script hash; do not track current DTO changes here.

use super::super::{
    ChapterShootingScriptV2, DirectorScriptPlan, ShootingBlockingV1, ShootingCameraAngleV1,
    ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingCoverageV1, ShootingDialogueCueV1,
    ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingScriptValidationError,
    ShootingShotSizeV1, ShootingSoundDesignV1, ShootingSpatialContinuityV1,
    ShootingTransitionContractV1, ShootingViewpointV1, ShootingVisualConstraintsV3,
    ShootingVisualPlanSourceV1,
};
use crate::chapter_visual_plan::{VisualSegmentProjectionV3, VisualUserAdditionV1};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChapterShootingScriptSchemaVersion {
    #[serde(rename = "chapter_shooting_script_v1")]
    V1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChapterShootingScriptV1 {
    pub schema_version: ChapterShootingScriptSchemaVersion,
    #[serde(default)]
    pub visual_plan_source: Option<ShootingVisualPlanSourceV1>,
    #[serde(default)]
    pub user_additions: Vec<VisualUserAdditionV1>,
    pub directing_statement: Option<ShootingDirectingStatementV1>,
    pub scenes: Vec<ShootingSceneV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShootingSceneV1 {
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
    #[serde(default)]
    pub visual_narrative_unit_id: Option<String>,
    #[serde(default)]
    pub user_addition_ids: Vec<String>,
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
    pub shots: Vec<ShootingShotV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShootingShotV1 {
    pub shot_index: u32,
    pub shot_id: String,
    pub title: String,
    pub source_span_ids: Vec<String>,
    #[serde(default)]
    pub visual_shot_id: Option<String>,
    #[serde(default)]
    pub planned_start_ms: Option<u64>,
    #[serde(default)]
    pub planned_end_ms: Option<u64>,
    #[serde(default)]
    pub user_addition_ids: Vec<String>,
    #[serde(default)]
    pub visual_constraints: Option<ShootingVisualConstraintsV3>,
    pub coverage: ShootingCoverageV1,
    pub narrative_purpose: String,
    pub subject: String,
    pub shot_size: ShootingShotSizeV1,
    pub camera_angle: ShootingCameraAngleV1,
    pub camera_height: ShootingCameraHeightV1,
    pub lens_intent: String,
    pub camera_movement: ShootingCameraMovementV1,
    pub composition: String,
    pub spatial_anchors: Vec<String>,
    pub blocking: ShootingBlockingV1,
    pub performance_beats: Vec<ShootingPerformanceBeatV1>,
    pub dialogue_cues: Vec<ShootingDialogueCueV1>,
    pub sound: ShootingSoundDesignV1,
    pub camera_start_state: String,
    pub camera_end_state: String,
    pub start_state: String,
    pub end_state: String,
    pub transition_in: String,
    pub transition_out: String,
    pub continuity_requirements: Vec<String>,
    pub estimated_natural_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShootingPerformanceBeatV1 {
    pub beat_index: u32,
    pub beat_id: String,
    pub source_span_ids: Vec<String>,
    #[serde(default)]
    pub user_addition_ids: Vec<String>,
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
    pub estimated_seconds: f64,
}

impl ChapterShootingScriptV1 {
    pub fn upgrade(&self) -> Result<ChapterShootingScriptV2, serde_json::Error> {
        let mut value = serde_json::to_value(self)?;
        super::remove_script_estimates(&mut value);
        value["schema_version"] = serde_json::json!(super::super::CHAPTER_SHOOTING_SCRIPT_V2);
        serde_json::from_value(value)
    }

    pub fn validate(&self) -> Result<(), ShootingScriptValidationError> {
        self.upgrade()
            .map_err(|error| ShootingScriptValidationError {
                path: "legacy.script".into(),
                message: error.to_string(),
            })?
            .validate()
    }

    pub fn shot_content_fingerprints(
        &self,
        plan: &DirectorScriptPlan,
    ) -> Result<BTreeMap<String, String>, serde_json::Error> {
        let mut fingerprints = BTreeMap::new();
        for scene in &self.scenes {
            let mut shared = scene.clone();
            shared.shots.clear();
            let mut shared = serde_json::to_value(shared)?;
            let object = shared.as_object_mut().expect("scene serializes as object");
            for field in [
                "scene_index",
                "scene_identity",
                "visual_narrative_unit_id",
                "sequence_index",
                "narrative_unit_index",
                "timeline_position",
                "title",
                "shots",
                "source_span_ids",
                "user_addition_ids",
            ] {
                object.remove(field);
            }
            for (value, transition) in object["transition_contracts"]
                .as_array_mut()
                .expect("transitions serialize as array")
                .iter_mut()
                .zip(&scene.transition_contracts)
            {
                // 未来单元尚未生成时也必须有稳定身份；不能由当前已完成场景推断。
                let target = transition
                    .to_narrative_unit_index
                    .checked_sub(1)
                    .and_then(|index| plan.narrative_units.get(index as usize))
                    .ok_or_else(|| {
                        <serde_json::Error as serde::ser::Error>::custom(format!(
                            "转场目标 {} 不在导演编排中",
                            transition.to_narrative_unit_index
                        ))
                    })?;
                value["to_narrative_unit_index"] = serde_json::to_value(&target.id)?;
            }
            for shot in &scene.shots {
                let mut content = shot.clone();
                content.shot_index = 0;
                content.title.clear();
                content.planned_end_ms = content
                    .planned_start_ms
                    .zip(content.planned_end_ms)
                    .map(|(start, end)| end.saturating_sub(start));
                content.planned_start_ms = content.planned_start_ms.map(|_| 0);
                content.visual_constraints = content
                    .visual_constraints
                    .as_ref()
                    .map(VisualSegmentProjectionV3::content_view);
                let additions = self
                    .user_additions
                    .iter()
                    .filter(|addition| {
                        shot.user_addition_ids.contains(&addition.id)
                            || shot
                                .performance_beats
                                .iter()
                                .any(|beat| beat.user_addition_ids.contains(&addition.id))
                            || (!addition.id.starts_with("director:")
                                && scene.user_addition_ids.contains(&addition.id))
                    })
                    .map(|addition| {
                        let mut addition = addition.clone();
                        addition.introduced_at_ms = None;
                        addition
                    })
                    .collect::<Vec<_>>();
                fingerprints.insert(shot.shot_id.clone(), crate::generation::canonical_json_sha256(
                    &serde_json::json!({ "scene": &shared, "shot": content, "user_additions": additions, "directing_statement": self.directing_statement })
                )?);
            }
        }
        Ok(fingerprints)
    }
}
