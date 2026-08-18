pub use a4n_domain::galgame::GalgamePipelineBeatRefV3;
use a4n_domain::shooting_script::ShootingCoverageV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameShootingScriptProjection {
    pub scenes: Vec<GalgamePipelineScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineScene {
    pub index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants: Vec<String>,
    pub summary: Option<String>,
    pub source_span_ids: Vec<String>,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub chapter_number: i32,
    pub shooting_scene_index: u32,
    pub beats: Vec<GalgamePipelineShootingBeat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineShootingBeat {
    pub shot_index: u32,
    pub beat_index: u32,
    pub coverage: ShootingCoverageV1,
    pub required: bool,
    pub objective: String,
    pub obstacle: String,
    pub visible_action: String,
    pub reaction: Option<String>,
    pub dialogue_intent: Option<String>,
    pub audience_question: String,
    pub answer_or_delay: String,
    pub entry_state: String,
    pub exit_state: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameScriptAdaptation {
    pub scene_lines: Vec<GalgamePipelineSceneLineDraft>,
    pub branch_scene_lines: Vec<GalgamePipelineBranchSceneLineDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineSceneLineDraft {
    pub scene_index: i32,
    pub lines: Vec<GalgamePipelineLineDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineLineDraft {
    pub index: i32,
    pub beat_ref: GalgamePipelineBeatRefV3,
    pub line_type: String,
    pub speaker_name: Option<String>,
    pub text: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineBranchSceneLineDraft {
    pub branch_id: String,
    pub lines: Vec<GalgamePipelineLineDraft>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct ShootingScriptProjectionRaw {
    pub(in crate::galgame::pipeline_json) scenes: Vec<SceneRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct SceneRaw {
    pub(in crate::galgame::pipeline_json) index: i32,
    pub(in crate::galgame::pipeline_json) title: String,
    pub(in crate::galgame::pipeline_json) location: Option<String>,
    pub(in crate::galgame::pipeline_json) time_of_day: Option<String>,
    pub(in crate::galgame::pipeline_json) participants: Vec<String>,
    pub(in crate::galgame::pipeline_json) summary: Option<String>,
    pub(in crate::galgame::pipeline_json) source_span_ids: Vec<String>,
    pub(in crate::galgame::pipeline_json) shooting_script_id: i64,
    pub(in crate::galgame::pipeline_json) shooting_script_revision: i32,
    pub(in crate::galgame::pipeline_json) chapter_number: i32,
    pub(in crate::galgame::pipeline_json) shooting_scene_index: u32,
    pub(in crate::galgame::pipeline_json) beats: Vec<GalgamePipelineShootingBeat>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct ScriptAdaptationRaw {
    pub(in crate::galgame::pipeline_json) scene_lines: Vec<SceneLineDraftRaw>,
    pub(in crate::galgame::pipeline_json) branch_scene_lines: Vec<BranchSceneLineDraftRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct SceneLineDraftRaw {
    pub(in crate::galgame::pipeline_json) scene_index: i32,
    pub(in crate::galgame::pipeline_json) lines: Vec<LineDraftRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct LineDraftRaw {
    pub(in crate::galgame::pipeline_json) index: i32,
    pub(in crate::galgame::pipeline_json) beat_ref: GalgamePipelineBeatRefV3,
    pub(in crate::galgame::pipeline_json) line_type: String,
    pub(in crate::galgame::pipeline_json) speaker_name: Option<String>,
    pub(in crate::galgame::pipeline_json) text: String,
    pub(in crate::galgame::pipeline_json) source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline_json) struct BranchSceneLineDraftRaw {
    pub(in crate::galgame::pipeline_json) branch_id: String,
    pub(in crate::galgame::pipeline_json) lines: Vec<LineDraftRaw>,
}
