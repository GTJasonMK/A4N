//! Pure chapter input contracts used by chapter-analysis workflows.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Analysis view for chapter-wide event browsing without loading source bodies.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct ChapterAnalysisEntry {
    pub id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub analysis_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResolvedChapterSource {
    pub requested_project_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub analysis_data_json: Option<Value>,
    pub content: String,
    pub content_sha256: String,
    pub source_revision_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterSourceSnapshot {
    pub project_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content_sha256: String,
    pub updated_at: NaiveDateTime,
}

pub trait ChapterAnalysisSource {
    fn has_available_analysis_content(&self) -> bool;
}

pub fn chapter_edit_revision(content: &str, title: Option<&str>) -> String {
    crate::generation::canonical_json_sha256(&(content, title))
        .expect("chapter content and title are JSON strings")
}

#[derive(Debug, Clone)]
pub struct StoreChapterAnalysisRecordInput {
    pub analysis_data: Value,
    pub summary: Option<String>,
    pub style_tags: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ImportChapterContentInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EditChapterContentInput {
    pub chapter_id: i64,
    pub expected_revision: String,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGeneratedChapterSceneSaveInput {
    pub chapter_id: i64,
    pub scene_number: i32,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChapterOutlineSyncDraft {
    pub chapter_number: i32,
    pub title: String,
    pub summary: String,
    pub beats: Option<serde_json::Value>,
    pub key_scenes: Option<serde_json::Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<serde_json::Value>,
    pub foreshadowing_payoff: Option<serde_json::Value>,
    pub characters_involved: Option<serde_json::Value>,
    pub word_count_target: Option<i32>,
}

pub fn required_outline_pacing_score(
    pacing_score: Option<f32>,
    chapter_number: i32,
) -> Result<f32, String> {
    let score = pacing_score
        .ok_or_else(|| format!("第 {chapter_number} 章缺少 pacing_score，无法分析节奏"))?;
    if score.is_finite() {
        Ok(score)
    } else {
        Err(format!(
            "第 {chapter_number} 章 pacing_score 不是有限数字，无法分析节奏"
        ))
    }
}
