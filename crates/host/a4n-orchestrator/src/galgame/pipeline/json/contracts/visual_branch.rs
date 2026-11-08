use a4n_domain::galgame::GalgameCgImportance;
use serde::{Deserialize, Serialize};

use super::scene_script::GalgamePipelineBeatRefV4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameVisualPlan {
    pub backgrounds: Vec<GalgamePipelineBackgroundPlan>,
    pub event_cgs: Vec<GalgamePipelineEventCgPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineBackgroundPlan {
    pub scene_index: i32,
    pub background_note: String,
    pub scene_profile_id: Option<i64>,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineEventCgPlan {
    pub scene_index: i32,
    pub line_index: i32,
    pub cg_note: String,
    pub importance: GalgameCgImportance,
    pub source_span_ids: Vec<String>,
}

pub use a4n_domain::media_authoring::{
    GalgameBranchDesign, GalgamePipelineChoiceOptionPlan, GalgamePipelineChoicePlan,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct VisualPlanRaw {
    pub(in crate::galgame::pipeline::json) backgrounds: Vec<BackgroundPlanRaw>,
    pub(in crate::galgame::pipeline::json) event_cgs: Vec<EventCgPlanRaw>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct BackgroundPlanRaw {
    pub(in crate::galgame::pipeline::json) scene_index: i32,
    pub(in crate::galgame::pipeline::json) background_note: String,
    pub(in crate::galgame::pipeline::json) scene_profile_id: Option<i64>,
    pub(in crate::galgame::pipeline::json) source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct EventCgPlanRaw {
    pub(in crate::galgame::pipeline::json) scene_index: i32,
    pub(in crate::galgame::pipeline::json) line_index: i32,
    pub(in crate::galgame::pipeline::json) cg_note: String,
    pub(in crate::galgame::pipeline::json) importance: GalgameCgImportance,
    pub(in crate::galgame::pipeline::json) source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct BranchDesignRaw {
    pub(in crate::galgame::pipeline::json) choices: Vec<ChoicePlanRaw>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct ChoicePlanRaw {
    pub(in crate::galgame::pipeline::json) after_beat_ref: GalgamePipelineBeatRefV4,
    pub(in crate::galgame::pipeline::json) question: String,
    pub(in crate::galgame::pipeline::json) options: Vec<ChoiceOptionPlanRaw>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::galgame::pipeline::json) struct ChoiceOptionPlanRaw {
    pub(in crate::galgame::pipeline::json) branch_id: String,
    pub(in crate::galgame::pipeline::json) option_text: String,
    pub(in crate::galgame::pipeline::json) branch_title: String,
    pub(in crate::galgame::pipeline::json) branch_objective: String,
    pub(in crate::galgame::pipeline::json) dramatic_action: String,
    pub(in crate::galgame::pipeline::json) reaction: String,
    pub(in crate::galgame::pipeline::json) player_knowledge: Vec<String>,
    pub(in crate::galgame::pipeline::json) returns_to:
        a4n_domain::media_authoring::AdaptationUnitRef,
}
