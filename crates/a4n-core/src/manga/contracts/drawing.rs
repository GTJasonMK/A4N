use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPageDrawingPromptV2 {
    pub schema_version: String,
    pub page_number: u32,
    pub image_prompt: String,
}
