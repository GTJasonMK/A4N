use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VideoPromptDownstreamDependencies {
    pub prompt_ids: Vec<i64>,
    pub image_ids: Vec<i64>,
    pub video_ids: Vec<i64>,
}

impl VideoPromptDownstreamDependencies {
    pub fn is_empty(&self) -> bool {
        self.prompt_ids.is_empty() && self.image_ids.is_empty() && self.video_ids.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChapterVideoPromptPatch {
    pub visual_prompt: Option<String>,
    pub motion_prompt: Option<Option<String>>,
    pub negative_prompt: Option<Option<String>>,
    pub duration_secs: Option<i32>,
    pub reference_image_ids_json: Option<Option<Value>>,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterVideoPromptSyncPolicy {
    pub preserve_generated_video_prompts: bool,
    pub preserve_frame_prompts_with_generated_images: bool,
}

#[derive(Debug, Clone)]
pub struct PreparedVideoPromptGenerationTrace {
    pub prompt_name: String,
    pub request_json: Value,
    pub rendered_messages_json: Value,
    pub response_text: String,
    pub workflow_run_id: Option<i64>,
    pub usage_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoPromptRecordFields {
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
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateChapterVideoPromptRecordInput {
    pub visual_prompt: Option<String>,
    pub motion_prompt: Option<Option<String>>,
    pub negative_prompt: Option<Option<String>>,
    pub duration_secs: Option<i32>,
    pub reference_image_ids_json: Option<Option<Value>>,
    pub metadata_json: Option<Value>,
    pub reset_video_binding_to_pending: bool,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoPromptSyncDraft {
    pub fields: ChapterVideoPromptRecordFields,
    pub generation_trace: Option<PreparedVideoPromptGenerationTrace>,
}
