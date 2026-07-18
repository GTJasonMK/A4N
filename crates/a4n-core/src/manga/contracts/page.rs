use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::panel::MangaPanelV2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPagePlanV3 {
    pub schema_version: String,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub pages: Vec<MangaPageV3>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPageV3 {
    pub page_number: u32,
    pub beat_refs: Vec<MangaShootingBeatRefV3>,
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

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct MangaShootingBeatRefV3 {
    pub scene_index: u32,
    pub shot_index: u32,
    pub beat_index: u32,
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
