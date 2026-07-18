use a4n_core::models::chapter::chapter_manga_prompt;
use serde_json::Value;

pub type MangaPromptUpdatedAt = chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct MangaPipelineStatusRows {
    pub page_prompts: Vec<chapter_manga_prompt::Model>,
    pub status_row: Option<chapter_manga_prompt::Model>,
}

#[derive(Debug, Clone)]
pub struct SetMangaPipelineStatusInput {
    pub chapter_id: i64,
    pub project_id: i64,
    pub pipeline_status: String,
    pub stage: String,
    pub current: i32,
    pub total: i32,
    pub message: String,
    pub task_id: Option<String>,
    pub allow_cancel_override: bool,
}

#[derive(Debug, Clone)]
pub struct SaveMangaPipelineCheckpointInput {
    pub status: SetMangaPipelineStatusInput,
    pub source_metadata: Value,
    pub reference_assets_metadata: Value,
    pub prompt_fingerprint: Value,
    pub checkpoint_data: Value,
}

/// 漫画提示词同步草稿（不含 LLM generation trace）。
#[derive(Debug, Clone)]
pub struct MangaPromptSyncDraft {
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub panel_number: i32,
    pub prompt_type: String,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub characters_in_frame: Option<Value>,
    pub layout: Option<Value>,
    pub camera_angle: Option<String>,
    pub dialogue_position: Option<String>,
    pub status: String,
}
