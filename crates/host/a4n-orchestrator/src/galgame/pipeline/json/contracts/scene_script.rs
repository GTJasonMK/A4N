pub use a4n_domain::galgame::GalgamePipelineBeatRefV4;
use a4n_domain::media_authoring::AdaptationBeat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameAdaptationProjection {
    pub scenes: Vec<GalgamePipelineScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineScene {
    pub index: i32,
    pub origin: a4n_domain::media_authoring::AdaptationOrigin,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants: Vec<String>,
    pub summary: Option<String>,
    pub source_span_ids: Vec<String>,
    pub adaptation_plan_id: i64,
    pub adaptation_plan_revision: i32,
    pub source_chapter_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub chapter_number: i32,
    pub unit_id: String,
    pub beats: Vec<AdaptationBeat>,
}

impl GalgamePipelineScene {
    pub fn beat_ref(&self, beat: &AdaptationBeat) -> GalgamePipelineBeatRefV4 {
        GalgamePipelineBeatRefV4 {
            source_chapter_id: self.source_chapter_id,
            unit_id: self.unit_id.clone(),
            beat_id: beat.id.clone(),
        }
    }

    pub fn contains_beat(&self, reference: &GalgamePipelineBeatRefV4) -> bool {
        reference.source_chapter_id == self.source_chapter_id
            && reference.unit_id == self.unit_id
            && self.beats.iter().any(|beat| beat.id == reference.beat_id)
    }
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
    pub origin: a4n_domain::media_authoring::AdaptationOrigin,
    pub beat_ref: GalgamePipelineBeatRefV4,
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
pub(in crate::galgame::pipeline::json) struct AdaptationProjectionRaw {
    pub(in crate::galgame::pipeline::json) scenes: Vec<SceneRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct SceneRaw {
    pub(in crate::galgame::pipeline::json) index: i32,
    pub(in crate::galgame::pipeline::json) origin: a4n_domain::media_authoring::AdaptationOrigin,
    pub(in crate::galgame::pipeline::json) title: String,
    pub(in crate::galgame::pipeline::json) location: Option<String>,
    pub(in crate::galgame::pipeline::json) time_of_day: Option<String>,
    pub(in crate::galgame::pipeline::json) participants: Vec<String>,
    pub(in crate::galgame::pipeline::json) summary: Option<String>,
    pub(in crate::galgame::pipeline::json) source_span_ids: Vec<String>,
    pub(in crate::galgame::pipeline::json) adaptation_plan_id: i64,
    pub(in crate::galgame::pipeline::json) adaptation_plan_revision: i32,
    pub(in crate::galgame::pipeline::json) source_chapter_id: i64,
    pub(in crate::galgame::pipeline::json) source_snapshot_chapter_id: i64,
    pub(in crate::galgame::pipeline::json) chapter_number: i32,
    pub(in crate::galgame::pipeline::json) unit_id: String,
    pub(in crate::galgame::pipeline::json) beats: Vec<AdaptationBeat>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct ScriptAdaptationRaw {
    pub(in crate::galgame::pipeline::json) scene_lines: Vec<SceneLineDraftRaw>,
    pub(in crate::galgame::pipeline::json) branch_scene_lines: Vec<BranchSceneLineDraftRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct SceneLineDraftRaw {
    pub(in crate::galgame::pipeline::json) scene_index: i32,
    pub(in crate::galgame::pipeline::json) lines: Vec<LineDraftRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct LineDraftRaw {
    pub(in crate::galgame::pipeline::json) index: i32,
    pub(in crate::galgame::pipeline::json) origin: a4n_domain::media_authoring::AdaptationOrigin,
    pub(in crate::galgame::pipeline::json) beat_ref: GalgamePipelineBeatRefV4,
    pub(in crate::galgame::pipeline::json) line_type: String,
    pub(in crate::galgame::pipeline::json) speaker_name: Option<String>,
    pub(in crate::galgame::pipeline::json) text: String,
    pub(in crate::galgame::pipeline::json) source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct BranchSceneLineDraftRaw {
    pub(in crate::galgame::pipeline::json) branch_id: String,
    pub(in crate::galgame::pipeline::json) lines: Vec<LineDraftRaw>,
}
