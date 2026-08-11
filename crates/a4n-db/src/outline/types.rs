use serde_json::Value;

use crate::generation::GenerationDbError;

#[derive(Debug, thiserror::Error)]
pub enum OutlineDbError {
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目状态 {current} 不允许执行：{operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Generation(#[from] GenerationDbError),
}

/// 分部大纲同步草稿（字段已解析，不再依赖 LLM JSON 形态）。
#[derive(Debug, Clone)]
pub struct PartOutlineSyncDraft {
    pub part_number: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub title: String,
    pub summary: String,
    pub theme: String,
    pub key_events: Value,
    pub character_focus: Value,
}

/// 章节大纲同步草稿（字段已解析）。
#[derive(Debug, Clone)]
pub struct ChapterOutlineSyncDraft {
    pub chapter_number: i32,
    pub title: String,
    pub summary: String,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: Option<i32>,
}
