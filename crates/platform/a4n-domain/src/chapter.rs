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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq, Default)]
pub struct ChapterStoryEventForeshadowingCounts {
    pub planted: usize,
    pub resolved: usize,
    pub tensions: usize,
}

/// Chapter analysis data reduced to the fields displayed by the story events page.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
pub struct ChapterStoryEventEntry {
    pub id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub analysis_data: Option<Value>,
    pub foreshadowing_counts: ChapterStoryEventForeshadowingCounts,
}

impl From<ChapterAnalysisEntry> for ChapterStoryEventEntry {
    fn from(mut entry: ChapterAnalysisEntry) -> Self {
        let mut foreshadowing_counts = ChapterStoryEventForeshadowingCounts::default();
        if let Some(Value::Object(analysis)) = entry.analysis_data.as_mut() {
            if let Some(Value::Object(foreshadowing)) = analysis.remove("foreshadowing") {
                foreshadowing_counts = ChapterStoryEventForeshadowingCounts {
                    planted: array_length(&foreshadowing, "planted"),
                    resolved: array_length(&foreshadowing, "resolved"),
                    tensions: array_length(&foreshadowing, "tensions"),
                };
            }
            if let Some(Value::Object(metadata)) = analysis.get_mut("metadata") {
                metadata.retain(|key, _| {
                    matches!(
                        key.as_str(),
                        "characters" | "locations" | "tone" | "timeline_marker"
                    )
                });
            }
            if let Some(Value::Object(summaries)) = analysis.get_mut("summaries") {
                summaries.retain(|key, _| {
                    matches!(
                        key.as_str(),
                        "compressed" | "summary" | "short" | "one_line" | "keywords"
                    )
                });
            }
            analysis
                .retain(|key, _| matches!(key.as_str(), "metadata" | "summaries" | "key_events"));
        }
        Self {
            id: entry.id,
            chapter_number: entry.chapter_number,
            title: entry.title,
            summary: entry.summary,
            analysis_data: entry.analysis_data,
            foreshadowing_counts,
        }
    }
}

fn array_length(object: &serde_json::Map<String, Value>, field: &str) -> usize {
    object
        .get(field)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

pub mod chapter_outline_status {
    pub const STATUS_DRAFT: &str = "draft";
    pub const STATUS_COMPLETED: &str = "completed";

    pub fn is_valid_status(status: &str) -> bool {
        matches!(status, STATUS_DRAFT | STATUS_COMPLETED)
    }
}

pub mod chapter_status {
    pub const STATUS_DRAFT: &str = "draft";
    pub const STATUS_GENERATING: &str = "generating";
    pub const STATUS_IMPORTED: &str = "imported";
    pub const STATUS_COMPLETED: &str = "completed";
    const STATUS_REVIEWED: &str = "reviewed";
    const STATUS_PUBLISHED: &str = "published";
    pub const STATUS_DELETED: &str = "deleted";

    pub fn is_valid_status(status: &str) -> bool {
        matches!(
            status,
            STATUS_DRAFT
                | STATUS_GENERATING
                | STATUS_IMPORTED
                | STATUS_COMPLETED
                | STATUS_REVIEWED
                | STATUS_PUBLISHED
                | STATUS_DELETED
        )
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterRecord {
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub status: String,
    pub summary: Option<String>,
    pub style_tags: Option<Value>,
    pub analysis_data: Option<Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl ChapterRecord {
    pub fn source_snapshot(&self) -> ChapterSourceSnapshot {
        ChapterSourceSnapshot {
            project_id: self.project_id,
            chapter_id: self.id,
            chapter_number: self.chapter_number,
            title: self.title.clone(),
            content_sha256: crate::source_text_hash(&self.content),
            updated_at: self.updated_at,
        }
    }
}

impl ChapterRecord {
    pub fn has_content(&self) -> bool {
        !self.content.trim().is_empty()
    }

    pub fn has_available_content(&self) -> bool {
        self.status != chapter_status::STATUS_DELETED && self.has_content()
    }

    pub fn is_generating(&self) -> bool {
        self.status == chapter_status::STATUS_GENERATING
    }
}

impl crate::source_spans::SourceChapter for ChapterRecord {
    fn source_chapter_id(&self) -> i64 {
        self.id
    }

    fn source_chapter_number(&self) -> i32 {
        self.chapter_number
    }

    fn source_content(&self) -> &str {
        &self.content
    }
}

impl ChapterAnalysisSource for ChapterRecord {
    fn has_available_analysis_content(&self) -> bool {
        self.has_available_content()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterSceneRecord {
    pub id: i64,
    pub chapter_id: i64,
    pub scene_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub word_count: i32,
    pub summary: Option<String>,
    pub scene_type: String,
    pub characters_present: Option<Value>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
    pub transition_from_previous: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartOutlineRecord {
    pub id: i64,
    pub project_id: i64,
    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub summary: Option<String>,
    pub theme: Option<String>,
    pub key_events: Option<Value>,
    pub character_focus: Option<Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterOutlineRecord {
    pub id: i64,
    pub project_id: i64,
    pub part_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: Option<i32>,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterOutlineProvenanceRecord {
    pub outline_id: i64,
    pub source_kind: String,
    pub source_fingerprint_json: Value,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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

#[cfg(test)]
mod tests {
    use super::{
        ChapterAnalysisEntry, ChapterRecord, ChapterStoryEventEntry,
        ChapterStoryEventForeshadowingCounts, chapter_outline_status, chapter_status,
    };
    use serde_json::json;

    #[test]
    fn chapter_outline_status_accepts_only_supported_values() {
        assert!(chapter_outline_status::is_valid_status("draft"));
        assert!(chapter_outline_status::is_valid_status("completed"));
        assert!(!chapter_outline_status::is_valid_status("planned"));
        assert!(!chapter_outline_status::is_valid_status(""));
    }

    #[test]
    fn chapter_status_and_content_availability_share_storage_values() {
        for status in [
            "draft",
            "generating",
            "imported",
            "completed",
            "reviewed",
            "published",
            "deleted",
        ] {
            assert!(chapter_status::is_valid_status(status));
        }
        assert!(!chapter_status::is_valid_status("unknown"));

        let mut chapter = ChapterRecord {
            id: 1,
            project_id: 2,
            outline_id: None,
            chapter_number: 1,
            title: None,
            content: "正文".into(),
            word_count: 2,
            status: chapter_status::STATUS_IMPORTED.into(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        };
        assert!(chapter.has_available_content());
        chapter.status = chapter_status::STATUS_DELETED.into();
        assert!(!chapter.has_available_content());
        chapter.status = chapter_status::STATUS_GENERATING.into();
        assert!(chapter.is_generating());
    }

    #[test]
    fn story_event_projection_keeps_display_fields_and_replaces_foreshadowing_with_counts() {
        let entry = ChapterAnalysisEntry {
            id: 9,
            chapter_number: 3,
            title: Some("第三章".into()),
            summary: Some("章节摘要".into()),
            analysis_data: Some(json!({
                "metadata": {
                    "characters": ["林岚"],
                    "locations": ["旧城"],
                    "items": ["omit item facts"],
                    "tags": ["omit tags"],
                    "tone": "紧张",
                    "timeline_marker": "第二天"
                },
                "summaries": {
                    "compressed": "压缩摘要",
                    "summary": "legacy summary",
                    "short": "legacy short",
                    "one_line": "单行摘要",
                    "keywords": ["线索"],
                    "unused": "omit"
                },
                "character_states": {"林岚": {"status": "omit"}},
                "world_facts": [{"statement": "omit"}],
                "foreshadowing": {
                    "planted": [{"description": "plant", "original_text": "large source"}],
                    "resolved": [{"resolution": "resolve"}],
                    "tensions": ["tension", "tension 2"],
                    "progressed": [{"resolution": "omit"}]
                },
                "key_events": [{"description": "事件", "type": "discovery"}],
                "unused_root": "omit"
            })),
        };

        let projected = ChapterStoryEventEntry::from(entry);

        assert_eq!(projected.id, 9);
        assert_eq!(projected.chapter_number, 3);
        assert_eq!(
            projected.analysis_data,
            Some(json!({
                "metadata": {
                    "characters": ["林岚"],
                    "locations": ["旧城"],
                    "tone": "紧张",
                    "timeline_marker": "第二天"
                },
                "summaries": {
                    "compressed": "压缩摘要",
                    "summary": "legacy summary",
                    "short": "legacy short",
                    "one_line": "单行摘要",
                    "keywords": ["线索"]
                },
                "key_events": [{"description": "事件", "type": "discovery"}]
            }))
        );
        assert_eq!(
            projected.foreshadowing_counts,
            ChapterStoryEventForeshadowingCounts {
                planted: 1,
                resolved: 1,
                tensions: 2,
            }
        );
    }

    #[test]
    fn story_event_projection_preserves_null_and_non_object_analysis_values() {
        for analysis_data in [None, Some(json!("legacy"))] {
            let projected = ChapterStoryEventEntry::from(ChapterAnalysisEntry {
                id: 1,
                chapter_number: 1,
                title: None,
                summary: None,
                analysis_data: analysis_data.clone(),
            });
            assert_eq!(projected.analysis_data, analysis_data);
            assert_eq!(
                projected.foreshadowing_counts,
                ChapterStoryEventForeshadowingCounts::default()
            );
        }
    }
}
mod model_output;
pub use model_output::{chapter_outline_output_schema, part_outline_output_schema};
