use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::generation_tasks::GenerationStartRequestMeta;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenerateSceneImageInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 100_000))]
    pub prompt: String,
    #[schemars(range(min = 1))]
    pub config_id: Option<i64>,
    #[schemars(range(min = 1))]
    pub chapter_id: Option<i64>,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneratePageImageInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    #[schemars(range(min = 1))]
    pub page_number: i32,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GenerateBatchImagesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1))]
    pub requests: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateGalgameImageInput {
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub prompt: String,
    pub image_type: String,
    pub config_id: Option<i64>,
    pub width: i32,
    pub height: i32,
    pub chapter_id: Option<i64>,
    pub generation_spec_id: i64,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub workflow_run_id: Option<i64>,
}

/// 图片生产生成的唯一启动合同。
///
/// 图片查询、上传和 provider 请求预览继续使用各自合同；只有生产生成
/// 统一经过 generation reservation/session/worker。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartImageGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub operation: ImageGenerationOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImageGenerationOperation {
    VisualPlanPoint {
        options: crate::chapter_visual_plan::VisualPlanPointGenerationOptions,
    },
    Scene {
        #[schemars(length(min = 1, max = 100_000))]
        prompt: String,
        #[schemars(range(min = 1))]
        config_id: Option<i64>,
        #[schemars(range(min = 1))]
        chapter_id: Option<i64>,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
    Page {
        #[schemars(range(min = 1))]
        chapter_number: i32,
        #[schemars(range(min = 1))]
        page_number: i32,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
    Batch {
        #[schemars(length(min = 1))]
        requests: Vec<Value>,
    },
    ObjectProfile {
        #[schemars(range(min = 1))]
        object_profile_id: i64,
        #[schemars(range(min = 1))]
        chapter_id: Option<i64>,
        #[schemars(length(max = 512))]
        style: Option<String>,
        #[schemars(range(min = 1))]
        config_id: Option<i64>,
        #[schemars(length(max = 128))]
        style_template_id: Option<String>,
        #[schemars(length(max = 512))]
        style_template_name: Option<String>,
        #[schemars(length(max = 128))]
        style_template_version: Option<String>,
        #[schemars(length(max = 100_000))]
        custom_style: Option<String>,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
    SceneProfile {
        #[schemars(range(min = 1))]
        scene_profile_id: i64,
        #[schemars(range(min = 1))]
        chapter_id: Option<i64>,
        #[schemars(length(max = 512))]
        style: Option<String>,
        #[schemars(range(min = 1))]
        config_id: Option<i64>,
        #[schemars(length(max = 128))]
        style_template_id: Option<String>,
        #[schemars(length(max = 512))]
        style_template_name: Option<String>,
        #[schemars(length(max = 128))]
        style_template_version: Option<String>,
        #[schemars(length(max = 100_000))]
        custom_style: Option<String>,
        #[schemars(range(min = 1))]
        width: i32,
        #[schemars(range(min = 1))]
        height: i32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileImageInput {
    #[schemars(range(min = 1))]
    pub object_profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: Option<i64>,
    pub style: Option<String>,
    pub config_id: Option<i64>,
    #[schemars(range(min = 1))]
    pub width: Option<i32>,
    #[schemars(range(min = 1))]
    pub height: Option<i32>,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ObjectProfileImagePromptOutput {
    pub object_profile_id: i64,
    pub object_name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileImageInput {
    #[schemars(range(min = 1))]
    pub scene_profile_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: Option<i64>,
    pub style: Option<String>,
    pub config_id: Option<i64>,
    #[schemars(range(min = 1))]
    pub width: Option<i32>,
    #[schemars(range(min = 1))]
    pub height: Option<i32>,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SceneProfileImagePromptOutput {
    pub scene_profile_id: i64,
    pub scene_name: String,
    pub prompt: String,
}

/// Host-facing boundary for profile image prompt generation.
///
/// Prompt construction only consumes API inputs and returns API outputs; it
/// does not require provider, file, task, or runtime state.
#[async_trait::async_trait]
pub trait ImageProfilePromptApplicationPort: Send + Sync {
    async fn object_profile_prompt(
        &self,
        user_id: i64,
        input: &ObjectProfileImageInput,
    ) -> Result<ObjectProfileImagePromptOutput, a4n_error::AppError>;

    async fn scene_profile_prompt(
        &self,
        user_id: i64,
        input: &SceneProfileImageInput,
    ) -> Result<SceneProfileImagePromptOutput, a4n_error::AppError>;
}

#[cfg(test)]
mod tests {
    use super::ObjectProfileImageInput;
    use serde_json::json;

    #[test]
    fn profile_image_prompt_contract_rejects_task_metadata() {
        assert!(
            serde_json::from_value::<ObjectProfileImageInput>(json!({
                "object_profile_id": 1,
                "task_id": "legacy-task"
            }))
            .is_err()
        );
    }
}
