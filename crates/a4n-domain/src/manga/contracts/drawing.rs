use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::versions::MangaPageDrawingPromptSchemaVersion;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MangaPageDrawingPromptV2 {
    pub schema_version: MangaPageDrawingPromptSchemaVersion,
    pub page_number: u32,
    pub image_prompt: String,
}
