use a4n_domain::galgame::{
    GalgameAudioRole, GalgameGenerationTargetKind, GalgameLinePresentationDocumentV2,
    GalgameLineType, GalgameMediaRole, GalgameMediaTargetKind, GalgamePipelineStartStage,
    GalgameSceneEntryKind, GalgameStagePositionV2, GalgameTargetEntityKind,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GalgameProjectInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartGalgamePipelineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 160))]
    pub task_id: String,
    #[serde(default)]
    pub start_from_stage: Option<GalgamePipelineStartStage>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreviewGalgamePipelineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    pub start_from_stage: Option<GalgamePipelineStartStage>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgamePipelineStatusSummary {
    pub run: Option<GalgameWorkflowRunSummary>,
    pub checkpoints: Vec<GalgameWorkflowCheckpointSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameWorkflowRunSummary {
    pub id: i64,
    pub status: String,
    pub mode: Option<String>,
    pub current_step_key: Option<String>,
    pub progress_json: Option<Value>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameWorkflowCheckpointSummary {
    pub id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameScriptDetail {
    pub id: i64,
    pub root_project_id: i64,
    pub title: String,
    pub speaker_profiles: Vec<GalgameSpeakerProfile>,
    pub text_anchors: Vec<GalgameTextAnchorSummary>,
    pub media_bindings: Vec<GalgameMediaBindingSummary>,
    pub scenes: Vec<GalgameSceneDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameSpeakerProfile {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameTextAnchorSummary {
    pub id: i64,
    pub source_snapshot_chapter_id: Option<i64>,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameSceneDetail {
    pub scene: GalgameScene,
    pub lines: Vec<GalgameLine>,
    pub choices: Vec<GalgameChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameScene {
    pub id: i64,
    pub source_snapshot_chapter_id: i64,
    pub scene_index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub background_note: Option<String>,
    pub entry_kind: GalgameSceneEntryKind,
    pub next_scene_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameLine {
    pub id: i64,
    pub scene_id: i64,
    pub line_index: i32,
    pub line_type: GalgameLineType,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<GalgameStagePositionV2>,
    pub presentation_json: GalgameLinePresentationDocumentV2,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameChoice {
    pub id: i64,
    pub line_id: i64,
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameMediaBindingSummary {
    pub binding: GalgameMediaBinding,
    pub asset: GalgameMediaAsset,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameMediaBinding {
    pub id: i64,
    pub media_asset_id: i64,
    pub target_kind: GalgameMediaTargetKind,
    pub target_id: i64,
    pub role: GalgameMediaRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameMediaAsset {
    pub id: i64,
    pub media_kind: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameAssetPlan {
    pub summary: GalgameAssetPlanSummary,
    pub backgrounds: Vec<GalgameAssetPlanItem>,
    pub portrait_expressions: Vec<GalgameAssetPlanItem>,
    pub event_cgs: Vec<GalgameAssetPlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct GalgameAssetPlanSummary {
    pub background_specs: usize,
    pub portrait_expression_specs: usize,
    pub event_cg_specs: usize,
    pub created_specs: usize,
    pub updated_specs: usize,
    pub reused_specs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GalgameAssetPlanItem {
    pub canonical_key: String,
    pub label: String,
    pub target_kind: GalgameGenerationTargetKind,
    pub target_entity_kind: GalgameTargetEntityKind,
    pub target_entity_id: i64,
    pub generation_spec_id: i64,
    pub status: String,
    pub outcome: GalgameAssetPlanOutcome,
    pub params_json: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GalgameAssetPlanOutcome {
    Created,
    Updated,
    Reused,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunGalgameAssetGenerationInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    pub generation_spec_ids: Vec<i64>,
    #[serde(default)]
    pub config_id: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameAssetGenerationItem {
    pub generation_spec_id: i64,
    pub target_kind: GalgameGenerationTargetKind,
    pub target_entity_kind: GalgameTargetEntityKind,
    pub target_entity_id: i64,
    pub status: String,
    pub prompt_artifact_id: Option<i64>,
    pub generated_image_id: Option<i64>,
    pub portrait_id: Option<i64>,
    pub media_asset_id: Option<i64>,
    pub binding_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GalgameAssetGenerationSummary {
    pub project_id: i64,
    pub generated_count: usize,
    pub skipped_completed_count: usize,
    pub items: Vec<GalgameAssetGenerationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BindGalgameMediaInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub media_asset_id: i64,
    pub target_kind: GalgameMediaTargetKind,
    #[schemars(range(min = 1))]
    pub target_id: i64,
    pub role: GalgameMediaRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UnbindGalgameMediaInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub target_kind: GalgameMediaTargetKind,
    #[schemars(range(min = 1))]
    pub target_id: i64,
    pub role: GalgameMediaRole,
    #[serde(default)]
    pub media_asset_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportGalgameAudioInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub target_line_id: i64,
    pub role: GalgameAudioRole,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameLineInput {
    #[schemars(range(min = 1))]
    pub line_id: i64,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "GalgameLineType")]
    pub line_type: Option<GalgameLineType>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<i64>")]
    pub speaker_profile_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub text: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "GalgameLinePresentationDocumentV2")]
    pub presentation: Option<GalgameLinePresentationDocumentV2>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<i64>")]
    pub text_anchor_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameSceneInput {
    #[schemars(range(min = 1))]
    pub scene_id: i64,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<String>")]
    pub location: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<String>")]
    pub time_of_day: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<String>")]
    pub background_note: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<i64>")]
    pub next_scene_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InsertGalgameLineInput {
    #[schemars(range(min = 1))]
    pub scene_id: i64,
    pub after_line_id: Option<i64>,
    pub line_type: GalgameLineType,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub presentation: GalgameLinePresentationDocumentV2,
    pub text_anchor_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteGalgameLineInput {
    #[schemars(range(min = 1))]
    pub line_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReorderGalgameLinesInput {
    #[schemars(range(min = 1))]
    pub scene_id: i64,
    pub line_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameChoicesInput {
    #[schemars(range(min = 1))]
    pub line_id: i64,
    pub choices: Vec<UpdateGalgameChoiceOptionInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGalgameChoiceOptionInput {
    pub option_text: String,
    #[schemars(range(min = 1))]
    pub target_scene_id: i64,
}
