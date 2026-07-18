use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

const IMAGE_CANCELLED_MESSAGE: &str = "图片生成已停止";
pub(in crate::actions::image_generation) const IMAGE_METADATA_SCHEMA_VERSION: u64 = 1;
pub(in crate::actions::image_generation) static DEFAULT_IMAGE_GENERATION_PROGRESS:
    ImageGenerationProgress<'static> = ImageGenerationProgress {
    reporter: None,
    cancel_token: None,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImageSummary {
    pub id: i64,
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub ai_score: Option<f32>,
    pub is_favorite: bool,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageGenerationTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectProfileImageInput {
    pub object_profile_id: i64,
    pub chapter_id: Option<i64>,
    pub style: Option<String>,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub style_template_id: Option<String>,
    #[serde(default)]
    pub style_template_name: Option<String>,
    #[serde(default)]
    pub style_template_version: Option<String>,
    #[serde(default)]
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObjectProfileImagePromptOutput {
    pub object_profile_id: i64,
    pub object_name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneProfileImageInput {
    pub scene_profile_id: i64,
    pub chapter_id: Option<i64>,
    pub style: Option<String>,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub style_template_id: Option<String>,
    #[serde(default)]
    pub style_template_name: Option<String>,
    #[serde(default)]
    pub style_template_version: Option<String>,
    #[serde(default)]
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SceneProfileImagePromptOutput {
    pub scene_profile_id: i64,
    pub scene_name: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateCanvasImageInput {
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    #[serde(default)]
    pub workflow_run_id: Option<i64>,
    pub prompt: String,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateGalgameImageInput {
    pub project_id: i64,
    pub prompt: String,
    pub image_type: String,
    pub config_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub generation_spec_id: i64,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: i64,
    pub workflow_run_id: Option<i64>,
}

type ImageGenerationProgressCallback<'a> = dyn Fn(u32, u32, &str) + Send + Sync + 'a;

pub struct ImageGenerationProgress<'a> {
    reporter: Option<&'a ImageGenerationProgressCallback<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ImageGenerationProgress<'a> {
    pub fn new(
        reporter: Option<&'a ImageGenerationProgressCallback<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub(in crate::actions::image_generation) fn progress(
        &self,
        current: u32,
        total: u32,
        message: &str,
    ) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    pub(in crate::actions::image_generation) fn check_cancelled(&self) -> Result<(), String> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(IMAGE_CANCELLED_MESSAGE.to_string());
        }
        Ok(())
    }

    pub(in crate::actions::image_generation) fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.cloned().unwrap_or_default()
    }
}
