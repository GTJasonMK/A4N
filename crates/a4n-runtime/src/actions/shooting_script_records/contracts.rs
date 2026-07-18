use a4n_core::{
    models::chapter::chapter_shooting_script, shooting_script::ChapterShootingScriptV1,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetChapterShootingScriptInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterShootingScriptInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditChapterShootingScriptRecordInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub record_id: i64,
    pub expected_revision: i32,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterShootingScriptView {
    pub record: chapter_shooting_script::Model,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditedChapterShootingScriptView {
    pub shooting_script: ChapterShootingScriptView,
    pub invalidated_project_ids: Vec<i64>,
}
