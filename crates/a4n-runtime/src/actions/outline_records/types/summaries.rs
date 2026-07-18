use a4n_core::models::chapter::{chapter_outline, part_outline};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartOutlineSummary {
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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationProgressSummary {
    pub total_parts: u64,
    pub completed_parts: u64,
    pub total_chapter_outlines: u64,
    pub completed_chapter_outlines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterOutlineSummary {
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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub(in crate::actions::outline_records) fn part_outline_summary(
    part: &part_outline::Model,
) -> PartOutlineSummary {
    PartOutlineSummary {
        id: part.id,
        project_id: part.project_id,
        part_number: part.part_number,
        title: part.title.clone(),
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        summary: part.summary.clone(),
        theme: part.theme.clone(),
        key_events: part.key_events.clone(),
        character_focus: part.character_focus.clone(),
        created_at: Some(part.created_at.to_string()),
        updated_at: Some(part.updated_at.to_string()),
    }
}

pub(in crate::actions::outline_records) fn chapter_outline_summary(
    outline: &chapter_outline::Model,
) -> ChapterOutlineSummary {
    ChapterOutlineSummary {
        id: outline.id,
        project_id: outline.project_id,
        part_id: outline.part_id,
        chapter_number: outline.chapter_number,
        title: outline.title.clone(),
        summary: outline.summary.clone(),
        beats: outline.beats.clone(),
        key_scenes: outline.key_scenes.clone(),
        pacing_score: outline.pacing_score,
        foreshadowing_setup: outline.foreshadowing_setup.clone(),
        foreshadowing_payoff: outline.foreshadowing_payoff.clone(),
        characters_involved: outline.characters_involved.clone(),
        word_count_target: outline.word_count_target,
        status: outline.status.clone(),
        created_at: Some(outline.created_at.to_string()),
        updated_at: Some(outline.updated_at.to_string()),
    }
}
