use a4n_core::{
    error::AppError,
    models::{
        character::protagonist_profile,
        project::{source_snapshot_chapter, text_anchor},
    },
};

use super::{
    super::{GalgamePipelineSourceSpan, GalgameSpeakerProfile},
    contracts::{GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan},
};

pub fn galgame_prompt_chapter_from_snapshot(
    chapter: &source_snapshot_chapter::Model,
    anchors: &[text_anchor::Model],
) -> Result<GalgamePipelinePromptChapter, AppError> {
    let mut source_spans = anchors
        .iter()
        .filter(|anchor| anchor.source_snapshot_chapter_id == Some(chapter.id))
        .map(|anchor| {
            let text = anchor.quote.trim();
            if text.is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame prompt text_anchor {} quote 不能为空",
                    anchor.id
                )));
            }
            Ok(GalgamePipelinePromptSourceSpan {
                id: galgame_source_span_id(chapter.chapter_number, anchor.id),
                text_anchor_id: anchor.id,
                source_snapshot_chapter_id: chapter.id,
                paragraph_index: anchor.paragraph_index,
                start_offset: anchor.start_offset,
                end_offset: anchor.end_offset,
                text: text.to_string(),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    source_spans.sort_by_key(|span| (span.paragraph_index, span.start_offset, span.text_anchor_id));
    if source_spans.is_empty() {
        return Err(AppError::InvalidState {
            current: format!("source_snapshot_chapter_{}_has_no_prompt_spans", chapter.id),
            operation: "build_galgame_pipeline_prompt_chapter".to_string(),
        });
    }
    Ok(GalgamePipelinePromptChapter {
        source_snapshot_chapter_id: chapter.id,
        source_chapter_id: chapter.source_chapter_id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        summary: chapter.summary.clone(),
        content_hash: chapter.content_hash.clone(),
        content: chapter.content.clone(),
        source_spans,
    })
}

pub fn galgame_pipeline_source_spans_from_prompt_chapters(
    chapters: &[GalgamePipelinePromptChapter],
) -> Vec<GalgamePipelineSourceSpan> {
    chapters
        .iter()
        .flat_map(|chapter| {
            chapter
                .source_spans
                .iter()
                .map(|span| GalgamePipelineSourceSpan {
                    id: span.id.clone(),
                    text_anchor_id: span.text_anchor_id,
                    source_snapshot_chapter_id: span.source_snapshot_chapter_id,
                })
        })
        .collect()
}

pub fn galgame_speaker_profiles_from_models(
    profiles: &[protagonist_profile::Model],
) -> Vec<GalgameSpeakerProfile> {
    profiles
        .iter()
        .map(|profile| GalgameSpeakerProfile {
            id: profile.id,
            name: profile.name.clone(),
        })
        .collect()
}

fn galgame_source_span_id(chapter_number: i32, text_anchor_id: i64) -> String {
    format!("ch{chapter_number}:ta{text_anchor_id}")
}
