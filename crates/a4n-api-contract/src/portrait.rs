use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::generation_tasks::GenerationStartRequestMeta;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortraitType {
    FullBody,
    HalfBody,
    Face,
    Chibi,
}

impl PortraitType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FullBody => "full_body",
            Self::HalfBody => "half_body",
            Self::Face => "face",
            Self::Chibi => "chibi",
        }
    }
}

impl TryFrom<&str> for PortraitType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "full_body" => Ok(Self::FullBody),
            "half_body" => Ok(Self::HalfBody),
            "face" => Ok(Self::Face),
            "chibi" => Ok(Self::Chibi),
            other => Err(format!("不支持的立绘类型: {other}")),
        }
    }
}

impl fmt::Display for PortraitType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortraitSummary {
    #[schemars(range(min = 1))]
    pub id: i64,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub portrait_type: PortraitType,
    pub image_path: String,
    pub prompt: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub is_primary: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortraitGenerationSeed {
    pub profile_name: String,
    pub prompt: String,
}

/// Host-facing portrait record boundary.
///
/// Authorization, persistence and ORM-to-summary projection stay in the
/// character/portrait owner; hosts consume only stable summaries.
#[async_trait::async_trait]
pub trait PortraitRecordApplicationPort: Send + Sync {
    async fn list_by_project(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PortraitSummary>, a4n_error::AppError>;

    async fn primary_by_profile(
        &self,
        user_id: i64,
        profile_id: i64,
    ) -> Result<Option<PortraitSummary>, a4n_error::AppError>;

    async fn by_id(
        &self,
        user_id: i64,
        portrait_id: i64,
    ) -> Result<Option<PortraitSummary>, a4n_error::AppError>;

    async fn set_primary(&self, user_id: i64, portrait_id: i64) -> Result<(), a4n_error::AppError>;

    async fn missing_generation_seeds(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PortraitGenerationSeed>, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListPortraitsByProjectInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetPrimaryPortraitInput {
    #[schemars(range(min = 1))]
    pub profile_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetPrimaryPortraitInput {
    #[schemars(range(min = 1))]
    pub portrait_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneratePortraitPromptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    pub portrait_type: PortraitType,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneratePortraitInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 512))]
    pub character_name: String,
    #[schemars(length(max = 512))]
    pub style: Option<String>,
    #[schemars(length(max = 512))]
    pub expression: Option<String>,
    #[schemars(length(max = 512))]
    pub pose: Option<String>,
    pub reference_image_ids: Vec<i64>,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePortraitInput {
    #[schemars(range(min = 1))]
    pub portrait_id: i64,
    pub reference_image_ids: Vec<i64>,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateAgeVariantInput {
    #[schemars(range(min = 1))]
    pub portrait_id: i64,
    #[schemars(length(min = 1, max = 128))]
    pub age: String,
    pub reference_image_ids: Vec<i64>,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateMissingPortraitsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UploadPortraitInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub profile_id: i64,
    #[schemars(length(min = 1, max = 512))]
    pub file_name: String,
    #[schemars(length(min = 1, max = 255))]
    pub mime_type: String,
    #[schemars(length(min = 1))]
    pub data_base64: String,
    pub portrait_type: PortraitType,
    #[schemars(length(min = 1))]
    pub prompt: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub set_primary: bool,
}

/// 人物图片生成的唯一启动合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartPortraitGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub operation: PortraitGenerationOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PortraitGenerationOperation {
    Generate {
        #[schemars(length(min = 1, max = 512))]
        character_name: String,
        #[schemars(length(max = 512))]
        style: Option<String>,
        #[schemars(length(max = 512))]
        expression: Option<String>,
        #[schemars(length(max = 512))]
        pose: Option<String>,
        reference_image_ids: Vec<i64>,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
    Regenerate {
        #[schemars(range(min = 1))]
        portrait_id: i64,
        reference_image_ids: Vec<i64>,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
    GenerateAgeVariant {
        #[schemars(range(min = 1))]
        portrait_id: i64,
        #[schemars(length(min = 1, max = 128))]
        age: String,
        reference_image_ids: Vec<i64>,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
    GenerateMissing {
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
}

#[cfg(test)]
mod tests {
    use super::PortraitType;

    #[test]
    fn portrait_type_rejects_unknown_values() {
        assert!(serde_json::from_str::<PortraitType>(r#""headshot""#).is_err());
    }
}
