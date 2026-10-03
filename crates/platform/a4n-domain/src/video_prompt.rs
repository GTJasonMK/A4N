use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROMPT_TYPE_VIDEO: &str = "video_prompt";
pub const PROMPT_TYPE_FRAME_SEGMENT_PLAN: &str = "frame_segment_plan";
pub const PROMPT_TYPE_FRAME: &str = "frame_prompt";
pub const PROMPT_TYPE_FIRST_FRAME: &str = "first_frame_prompt";
pub const PROMPT_TYPE_CHAT_VIDEO_FRAME: &str = "chat_video_frame_prompt";
pub const PROMPT_TYPE_STORYBOARD: &str = "storyboard_prompt";
pub const WORKFLOW_MODE_SHARED_SEGMENT_PLAN: &str = "shared_segment_plan";
pub const WORKFLOW_MODE_STORYBOARD: &str = "storyboard";
pub const WORKFLOW_MODE_BOUNDARY_FRAMES: &str = "boundary_frames";
pub const WORKFLOW_MODE_FIRST_FRAME_CHAIN: &str = "first_frame_chain";
pub const WORKFLOW_MODE_CHAT_VIDEO: &str = "chat_video";
pub const WORKFLOW_MODES: [&str; 5] = [
    WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
    WORKFLOW_MODE_BOUNDARY_FRAMES,
    WORKFLOW_MODE_FIRST_FRAME_CHAIN,
    WORKFLOW_MODE_CHAT_VIDEO,
    WORKFLOW_MODE_STORYBOARD,
];
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_INTERRUPTED: &str = "interrupted";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_CANCELLED: &str = "cancelled";

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterVideoPromptRecord {
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
    pub timeline_item_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<&ChapterVideoPromptRecord> for ChapterVideoPromptSummary {
    fn from(prompt: &ChapterVideoPromptRecord) -> Self {
        Self {
            id: prompt.id,
            project_id: prompt.project_id,
            chapter_id: prompt.chapter_id,
            style: prompt.style.clone(),
            prompt_type: prompt.prompt_type.clone(),
            workflow_mode: prompt.workflow_mode.clone(),
            shot_number: prompt.shot_number,
            title: prompt.title.clone(),
            scene_summary: prompt.scene_summary.clone(),
            visual_prompt: prompt.visual_prompt.clone(),
            motion_prompt: prompt.motion_prompt.clone(),
            negative_prompt: prompt.negative_prompt.clone(),
            duration_secs: prompt.duration_secs,
            aspect_ratio: prompt.aspect_ratio.clone(),
            reference_image_ids_json: prompt.reference_image_ids_json.clone(),
            status: prompt.status.clone(),
            generated_video_id: prompt.generated_video_id,
            metadata_json: prompt.metadata_json.clone(),
            created_at: prompt.created_at.to_string(),
            updated_at: prompt.updated_at.to_string(),
        }
    }
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
