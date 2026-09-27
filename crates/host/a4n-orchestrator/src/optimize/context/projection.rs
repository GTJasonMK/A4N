use a4n_chapter_analysis::summary_text;
use a4n_db_models::models::chapter::{chapter, part_outline};
use a4n_db_models::models::character::protagonist_profile;
use a4n_db_models::models::project::novel_project;
use serde_json::{Value, json};

use a4n_db::{
    optimize::{OptimizationChapterOutlineRecord, OptimizationChapterSummaryRecord},
    rag::RagForeshadowingRow,
};
use a4n_domain::source_spans::source_span_prompt_values;
use a4n_domain::source_spans::source_spans_for_chapters;

pub(super) fn optimization_project_context(project: &novel_project::Model) -> Value {
    json!({
        "id": project.id,
        "title": project.title,
        "description": project.description,
        "project_type": project.project_type,
        "status": project.status,
        "target_chapters": project.target_chapters,
        "current_chapter": project.current_chapter,
        "total_chapters": project.total_chapters,
        "chapters_per_part": project.chapters_per_part,
        "raw_blueprint": project.raw_blueprint.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "export_config": project.export_config.as_deref().map(str::trim).filter(|text| !text.is_empty()),
    })
}

pub(super) fn optimization_chapter_context(ch: &chapter::Model) -> Value {
    let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
    json!({
        "chapter_number": ch.chapter_number,
        "content": ch.content,
        "title": ch.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": optimization_chapter_summary_parts(ch.analysis_data.as_ref(), ch.summary.as_deref()),
        "word_count": ch.word_count,
        "status": ch.status,
        "source_spans": source_span_prompt_values(&source_spans),
    })
}

pub(super) fn optimization_chapter_summary_context(ch: &OptimizationChapterSummaryRecord) -> Value {
    json!({
        "chapter_number": ch.chapter_number,
        "title": ch.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": optimization_chapter_summary_parts(ch.analysis_data.as_ref(), ch.summary.as_deref()),
        "word_count": ch.word_count,
    })
}

fn optimization_chapter_summary_parts(
    analysis_data: Option<&Value>,
    summary: Option<&str>,
) -> Option<String> {
    analysis_data
        .as_ref()
        .and_then(|value| summary_text(value))
        .or_else(|| {
            summary
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(ToOwned::to_owned)
        })
}

pub(super) fn optimization_outline_context(outline: &OptimizationChapterOutlineRecord) -> Value {
    json!({
        "chapter_number": outline.chapter_number,
        "title": outline.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": outline.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "beats": outline.beats,
        "key_scenes": outline.key_scenes,
        "pacing_score": outline.pacing_score,
        "foreshadowing_setup": outline.foreshadowing_setup,
        "foreshadowing_payoff": outline.foreshadowing_payoff,
        "characters_involved": outline.characters_involved,
        "word_count_target": outline.word_count_target,
    })
}

pub(super) fn optimization_part_outline_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary.as_deref().map(str::trim).filter(|summary| !summary.is_empty()),
        "theme": part.theme.as_deref().map(str::trim).filter(|theme| !theme.is_empty()),
        "key_events": part.key_events,
        "character_focus": part.character_focus,
    })
}

pub(super) fn optimization_character_profile_context(
    profile: &protagonist_profile::Model,
) -> Value {
    json!({
        "name": profile.name,
        "gender": profile.gender,
        "age": profile.age,
        "base_attributes": profile.base_attributes,
        "current_state": profile.current_state,
        "character_arc": profile.character_arc,
        "voice_fingerprint": profile.voice_fingerprint,
        "appearance": profile.appearance.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "background": profile.background.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "personality": profile.personality.as_deref().map(str::trim).filter(|text| !text.is_empty()),
        "last_synced_chapter": profile.last_synced_chapter,
    })
}

pub(super) fn optimization_foreshadowing_context(item: &RagForeshadowingRow) -> Value {
    json!({
        "description": item.description,
        "setup_chapter": item.setup_chapter,
        "payoff_chapter": item.payoff_chapter,
        "status": item.status,
        "related_characters": item.related_characters,
        "importance": item.importance,
    })
}
