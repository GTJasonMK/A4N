use super::*;

pub type VideoGenerationProgressCallback<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_id: i64,
    #[serde(default)]
    pub config_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideosBatchInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_ids: Vec<i64>,
    #[serde(default)]
    pub config_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateChapterVideosBatchResult {
    pub videos: Vec<GeneratedVideoSummary>,
    pub success_count: usize,
    pub failed_count: usize,
    pub pending_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReferenceMediaVideoInput {
    pub prompt: String,
    #[serde(default)]
    pub image_files: Vec<String>,
    #[serde(default)]
    pub video_files: Vec<String>,
    #[serde(default)]
    pub audio_files: Vec<String>,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    #[serde(default)]
    pub config_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReferenceMediaVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub input: ReferenceMediaVideoInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoGenerationTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewChapterVideoPromptOverrideInput {
    #[serde(default)]
    pub visual_prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub motion_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub negative_prompt: Option<Option<String>>,
    #[serde(default)]
    pub duration_secs: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub reference_image_ids_json: Option<Option<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateCanvasVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node_id: i64,
    #[serde(default)]
    pub workflow_run_id: Option<i64>,
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub config_id: Option<i64>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub duration_secs: Option<i32>,
    #[serde(default)]
    pub fps: Option<i32>,
    #[serde(default)]
    pub first_frame_image_id: Option<i64>,
    #[serde(default)]
    pub last_frame_image_id: Option<i64>,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
    #[serde(default)]
    pub source_node_ids: Vec<i64>,
}

fn deserialize_nullable_update_field<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedVideoSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub file_size: Option<i64>,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub status: String,
    pub provider_job_id: Option<String>,
    pub error_message: Option<String>,
    pub metadata_json: Option<Value>,
    pub is_favorite: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

mod progress;
pub(crate) use progress::VideoGenerationProgress;

mod summary;
pub(crate) use summary::video_to_summary;
