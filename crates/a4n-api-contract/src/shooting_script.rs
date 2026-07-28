use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetChapterShootingScriptInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterShootingScriptInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EditChapterShootingScriptRecordInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub record_id: i64,
    pub expected_revision: i32,
    pub script: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterShootingScriptView {
    pub record: Value,
    pub script: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EditedChapterShootingScriptView {
    pub shooting_script: ChapterShootingScriptView,
    pub invalidated_project_ids: Vec<i64>,
}
