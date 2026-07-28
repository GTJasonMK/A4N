use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportUploadedFileInput {
    pub file_name: String,
    pub file_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ParsedChapterSummary {
    pub number: i32,
    pub title: String,
    pub content: String,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ImportFileResult {
    pub chapters: Vec<ParsedChapterSummary>,
    pub detected_encoding: String,
    pub total_words: i32,
    pub warnings: Vec<String>,
}
