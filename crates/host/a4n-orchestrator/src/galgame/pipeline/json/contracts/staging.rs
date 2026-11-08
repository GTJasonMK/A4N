pub use a4n_domain::galgame::GalgameLinePresentationV2;
use serde::{Deserialize, Serialize};

use super::scene_script::GalgamePipelineBeatRefV4;

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
    pub origin: a4n_domain::media_authoring::AdaptationOrigin,
    pub beat_ref: GalgamePipelineBeatRefV4,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct CharacterStagingRaw {
    pub(in crate::galgame::pipeline::json) scene_lines: Vec<SceneLineStagedRaw>,
    pub(in crate::galgame::pipeline::json) branch_scene_lines: Vec<BranchSceneLineStagedRaw>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct SceneLineStagedRaw {
    pub(in crate::galgame::pipeline::json) scene_index: i32,
    pub(in crate::galgame::pipeline::json) lines: Vec<LineStagedRaw>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct LineStagedRaw {
    pub(in crate::galgame::pipeline::json) index: i32,
    pub(in crate::galgame::pipeline::json) origin: a4n_domain::media_authoring::AdaptationOrigin,
    pub(in crate::galgame::pipeline::json) beat_ref: GalgamePipelineBeatRefV4,
    pub(in crate::galgame::pipeline::json) line_type: String,
    pub(in crate::galgame::pipeline::json) speaker_profile_id: Option<i64>,
    pub(in crate::galgame::pipeline::json) speaker_name: Option<String>,
    pub(in crate::galgame::pipeline::json) text: String,
    pub(in crate::galgame::pipeline::json) presentation: GalgameLinePresentationV2,
    pub(in crate::galgame::pipeline::json) source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct BranchSceneLineStagedRaw {
    pub(in crate::galgame::pipeline::json) branch_id: String,
    pub(in crate::galgame::pipeline::json) lines: Vec<LineStagedRaw>,
}
