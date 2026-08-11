use serde_json::Value;

pub type MangaPromptUpdatedAt = chrono::NaiveDateTime;

/// 整页漫画提示词同步草稿（不含 LLM generation trace）。
#[derive(Debug, Clone)]
pub struct MangaPagePromptSyncDraft {
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub characters_in_frame: Option<Value>,
    pub layout: Option<Value>,
    pub status: String,
}
