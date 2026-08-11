use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProjectAnalysisSummary {
    pub progress: Value,
    pub analyzed_chapters: usize,
}

#[derive(Debug, Clone)]
pub struct ParsedChapterInput {
    pub number: i32,
    pub title: String,
    pub content: String,
}
