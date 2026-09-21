pub use a4n_domain::galgame::GalgameLinePresentationV2;
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct CharacterStagingRaw {
    pub(in crate::galgame::pipeline_json) scene_lines: Vec<SceneLineStagedRaw>,
    pub(in crate::galgame::pipeline_json) branch_scene_lines: Vec<BranchSceneLineStagedRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct SceneLineStagedRaw {
    pub(in crate::galgame::pipeline_json) scene_index: i32,
    pub(in crate::galgame::pipeline_json) lines: Vec<LineStagedRaw>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct BranchSceneLineStagedRaw {
    pub(in crate::galgame::pipeline_json) branch_id: String,
    pub(in crate::galgame::pipeline_json) lines: Vec<LineStagedRaw>,
}
