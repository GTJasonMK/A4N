use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ImportChapterInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChapterContentInput {
    pub chapter_id: i64,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportedChapterSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub status: String,
    pub word_count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterSummary {
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub status: String,
    pub summary: Option<String>,
    pub style_tags: Option<serde_json::Value>,
    pub analysis_data: Option<serde_json::Value>,
    pub analysis_sync_status: Option<String>,
    pub analysis_sync_can_resume: bool,
    pub analysis_sync_resume_block_reason: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
