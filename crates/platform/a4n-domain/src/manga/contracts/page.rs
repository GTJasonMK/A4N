use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::panel::MangaPanelV2;
use super::versions::MangaPagePlanSchemaVersion;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPagePlanV4 {
    pub schema_version: MangaPagePlanSchemaVersion,
    pub adaptation_plan_id: i64,
    pub adaptation_plan_revision: i32,
    pub pages: Vec<MangaPageV4>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPageV4 {
    pub page_number: u32,
    pub beat_refs: Vec<crate::media_authoring::AdaptationBeatRef>,
    pub role: MangaPageRoleV2,
    pub pacing: MangaPagePacingV2,
    pub importance: MangaPageImportanceV2,
    pub reading_flow: MangaReadingFlowV2,
    pub page_goal: String,
    pub visual_thesis: String,
    pub entry_state: String,
    pub exit_state: String,
    pub dominant_panel_number: u32,
    pub panels: Vec<MangaPanelV2>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaPageRoleV2 {
    Opening,
    Setup,
    Rising,
    Climax,
    Falling,
    Resolution,
    Transition,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaPagePacingV2 {
    Slow,
    Medium,
    Fast,
    Explosive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaPageImportanceV2 {
    Critical,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MangaReadingFlowV2 {
    TopToBottomLeftToRight,
    ZPath,
    SPath,
    Diagonal,
}
