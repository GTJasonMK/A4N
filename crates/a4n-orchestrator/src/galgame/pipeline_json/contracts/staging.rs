use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::scene_script::GalgamePipelineBeatRefV3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameCharacterStaging {
    pub scene_lines: Vec<GalgamePipelineSceneLineStaged>,
    pub branch_scene_lines: Vec<GalgamePipelineBranchSceneLineStaged>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineSceneLineStaged {
    pub scene_index: i32,
    pub lines: Vec<GalgamePipelineLineStaged>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineLineStaged {
    pub index: i32,
    pub beat_ref: GalgamePipelineBeatRefV3,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub speaker_name: Option<String>,
    pub text: String,
    pub presentation: GalgameLinePresentationV2,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineBranchSceneLineStaged {
    pub branch_id: String,
    pub lines: Vec<GalgamePipelineLineStaged>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameLinePresentationV2 {
    pub schema_version: String,
    pub visible_cast: Vec<GalgameVisibleCastMemberV2>,
    pub enter: Vec<i64>,
    pub exit: Vec<i64>,
    pub focus_character_profile_id: Option<i64>,
    pub hold: bool,
    pub player_knowledge: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameLinePresentationDocumentV2 {
    pub schema_version: String,
    pub visible_cast: Vec<GalgameVisibleCastMemberV2>,
    pub enter: Vec<i64>,
    pub exit: Vec<i64>,
    pub focus_character_profile_id: Option<i64>,
    pub hold: bool,
    pub player_knowledge: Vec<String>,
    pub cg: Option<bool>,
    pub cg_note: Option<String>,
    pub cg_importance: Option<String>,
    pub transition: Option<String>,
    pub branch: Option<bool>,
    pub branch_id: Option<String>,
    pub returns_to_scene_index: Option<i32>,
    pub after_beat_ref: Option<GalgamePipelineBeatRefV3>,
}

impl GalgameLinePresentationDocumentV2 {
    pub fn directing_presentation(&self) -> GalgameLinePresentationV2 {
        GalgameLinePresentationV2 {
            schema_version: self.schema_version.clone(),
            visible_cast: self.visible_cast.clone(),
            enter: self.enter.clone(),
            exit: self.exit.clone(),
            focus_character_profile_id: self.focus_character_profile_id,
            hold: self.hold,
            player_knowledge: self.player_knowledge.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameVisibleCastMemberV2 {
    pub character_profile_id: i64,
    pub expression: String,
    pub pose: String,
    pub position: GalgameStagePositionV2,
    pub facing: GalgameFacingV2,
    pub layer: GalgameStageLayerV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameStagePositionV2 {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameFacingV2 {
    Left,
    Right,
    Front,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GalgameStageLayerV2 {
    Back,
    Middle,
    Front,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct CharacterStagingRaw {
    pub(in crate::galgame::pipeline_json) scene_lines: Vec<SceneLineStagedRaw>,
    pub(in crate::galgame::pipeline_json) branch_scene_lines: Vec<BranchSceneLineStagedRaw>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct SceneLineStagedRaw {
    pub(in crate::galgame::pipeline_json) scene_index: i32,
    pub(in crate::galgame::pipeline_json) lines: Vec<LineStagedRaw>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct LineStagedRaw {
    pub(in crate::galgame::pipeline_json) index: i32,
    pub(in crate::galgame::pipeline_json) beat_ref: GalgamePipelineBeatRefV3,
    pub(in crate::galgame::pipeline_json) line_type: String,
    pub(in crate::galgame::pipeline_json) speaker_profile_id: Option<i64>,
    pub(in crate::galgame::pipeline_json) speaker_name: Option<String>,
    pub(in crate::galgame::pipeline_json) text: String,
    pub(in crate::galgame::pipeline_json) presentation: GalgameLinePresentationV2,
    pub(in crate::galgame::pipeline_json) source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct BranchSceneLineStagedRaw {
    pub(in crate::galgame::pipeline_json) branch_id: String,
    pub(in crate::galgame::pipeline_json) lines: Vec<LineStagedRaw>,
}
