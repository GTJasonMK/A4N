use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MANGA_PAGE_PLAN_V4: &str = "manga_page_plan_v4";
pub const MANGA_PAGE_DRAWING_PROMPT_V2: &str = "manga_page_drawing_prompt_v2";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum MangaPagePlanSchemaVersion {
    #[serde(rename = "manga_page_plan_v4")]
    V4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum MangaPageDrawingPromptSchemaVersion {
    #[serde(rename = "manga_page_drawing_prompt_v2")]
    V2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum MangaPageGenerationSchemaVersion {
    #[serde(rename = "manga_page_generation_v3")]
    V3,
}
