use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProjectAnalysisSummary {
    pub progress: Value,
    pub analyzed_chapters: usize,
    pub outlined_chapters: usize,
    pub generated_parts: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartImportAnalysisInput {
    pub filename: String,
    pub file_type: String,
    pub detected_encoding: Option<String>,
    pub chapters: Vec<ParsedChapterInput>,
    pub dedup_paragraphs: i32,
    pub dedup_rate: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedChapterInput {
    pub number: i32,
    pub title: String,
    pub content: String,
    pub content_hash: Option<String>,
}
