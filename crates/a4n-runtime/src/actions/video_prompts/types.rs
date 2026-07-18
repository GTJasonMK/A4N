use super::*;

pub(super) const VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
pub(super) const VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
pub(super) const VIDEO_WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
pub(super) const VIDEO_WORKFLOW_MODE_STORYBOARD: &str = "storyboard";
pub(super) const PREVIEW_STYLE_PLACEHOLDER: &str =
    "未指定风格（请求预览占位；真实生成需要填写 style）";
pub(super) const PREVIEW_CHAPTER_TITLE_PLACEHOLDER: &str =
    "未命名章节（请求预览占位；真实生成需要章节标题）";
pub(super) const PREVIEW_CHAPTER_CONTENT_PLACEHOLDER: &str = "【请求预览占位正文】当前章节正文为空。真实生成前需要填写章节正文；此占位仅用于展示将发送给模型的请求结构。";
pub(super) const CHAPTER_VIDEO_PROMPT_WORKFLOW_MODES: &[&str] = &[
    VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_STORYBOARD,
];
pub(super) const MAX_NON_CHAT_REFERENCE_IMAGE_IDS: usize = 4;
pub(super) const MAX_CHAT_REFERENCE_IMAGE_IDS: usize = 16;
pub type VideoPromptProgress<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);
pub type VideoPromptChunk<'a> = &'a mut (dyn FnMut(&str, u32) + Send);

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratePromptOptionsInput {
    #[serde(default)]
    pub workflow_mode: Option<String>,
    pub style: String,
    #[serde(default)]
    pub prompt_output_language: Option<String>,
    pub prompt_batch_size: i32,
    pub duration_secs: i32,
    #[serde(default)]
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    #[serde(default)]
    pub video_library_image_ids: Vec<i64>,
    #[serde(default)]
    pub regenerate_existing: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoPromptInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub options: GeneratePromptOptionsInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPromptTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterVideoPromptSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: Option<String>,
    pub prompt_type: String,
    pub workflow_mode: String,
    pub shot_number: i32,
    pub title: Option<String>,
    pub scene_summary: Option<String>,
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub duration_secs: i32,
    pub aspect_ratio: Option<String>,
    pub reference_image_ids_json: Option<Value>,
    pub status: String,
    pub generated_video_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterVideoPromptInput {
    pub visual_prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub motion_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub negative_prompt: Option<Option<String>>,
    pub duration_secs: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    pub reference_image_ids_json: Option<Option<Value>>,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteChapterVideoPromptGroupResult {
    pub deleted_prompt_count: u64,
    pub deleted_image_count: u64,
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

pub(super) struct PreparedChapterVideoPromptPreview {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) workflow_mode: String,
    pub(super) source: ChapterVideoSource,
    pub(super) vars: HashMap<String, Value>,
    pub(super) prompt_options: ChapterVideoPromptOptions,
    pub(super) placeholder_notes: Vec<String>,
}

pub(super) struct ChapterVideoPromptGenerationSpec {
    pub(super) initial_stop_message: &'static str,
    pub(super) operation: &'static str,
    pub(super) empty_content_message: &'static str,
    pub(super) running_message: &'static str,
    pub(super) workflow_operation: &'static str,
    pub(super) workflow_kind: &'static str,
    pub(super) workflow_step_key: &'static str,
}

pub(super) struct PreparedChapterVideoPromptGeneration {
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) workflow_mode: String,
    pub(super) source: ChapterVideoSource,
    pub(super) vars: HashMap<String, Value>,
    pub(super) prompt_options: ChapterVideoPromptOptions,
    pub(super) workflow_run_id: i64,
}

pub(super) struct VideoPromptMutationPlan {
    pub(super) allow_downstream: bool,
    pub(super) delete_dependencies: Option<VideoPromptDownstreamDependencies>,
}
