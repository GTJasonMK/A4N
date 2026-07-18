use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type PortraitProgress<'a> = &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

#[derive(Debug, Clone, Serialize)]
pub struct PortraitSummary {
    pub id: i64,
    pub profile_id: i64,
    pub project_id: i64,
    pub portrait_type: String,
    pub image_path: String,
    pub prompt: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub is_primary: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePortraitPromptInput {
    pub project_id: i64,
    pub profile_id: i64,
    pub portrait_type: String,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratePortraitInput {
    pub project_id: i64,
    pub character_name: String,
    pub style: Option<String>,
    pub expressions: Option<Vec<String>>,
    #[serde(default)]
    pub pose: Option<String>,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegeneratePortraitInput {
    pub portrait_id: i64,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateAgeVariantInput {
    pub portrait_id: i64,
    pub age: String,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateMissingPortraitsInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadPortraitInput {
    pub project_id: i64,
    pub profile_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub portrait_type: String,
    pub prompt: String,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub pose: Option<String>,
    #[serde(default)]
    pub set_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortraitTaskScope {
    pub project_id: i64,
    pub total: u32,
}
