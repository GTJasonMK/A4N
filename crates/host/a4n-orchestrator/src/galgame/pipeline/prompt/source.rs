use a4n_db::galgame::{
    GalgamePipelinePromptChapterSourceRecord, GalgamePipelinePromptSpeakerProfileRecord,
    GalgamePipelinePromptTextAnchorRecord,
};
use a4n_error::AppError;
use serde_json::Value;

use crate::source_markers::SourceMarkerMap;

use super::{
    super::super::{GalgamePipelineSourceSpan, GalgameSpeakerProfile},
    contracts::{GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan},
};

pub(crate) fn galgame_source_markers(
    chapters: &[GalgamePipelinePromptChapter],
) -> Result<SourceMarkerMap, AppError> {
    let chapter_values = serde_json::to_value(chapters)?;
    SourceMarkerMap::from_ids_and_values(
        chapters
            .iter()
            .flat_map(|chapter| chapter.source_spans.iter().map(|span| span.id.clone())),
        std::iter::once(&chapter_values),
    )
}

pub(super) fn marked_galgame_prompt_chapters(
    chapters: &[GalgamePipelinePromptChapter],
    source_markers: &SourceMarkerMap,
) -> Result<Value, AppError> {
    let mut marked = serde_json::to_value(chapters)?;
    let Some(marked_chapters) = marked.as_array_mut() else {
        return Err(AppError::InvalidArgument(
            "Galgame prompt chapters 必须是数组".into(),
        ));
    };
    for (marked_chapter, chapter) in marked_chapters.iter_mut().zip(chapters) {
        let ranges = chapter
            .source_spans
            .iter()
            .map(|span| {
                Ok((
                    span.id.clone(),
                    usize::try_from(span.start_offset).map_err(|_| {
                        AppError::InvalidArgument(format!(
                            "Galgame source span {} 起点无效",
                            span.id
                        ))
                    })?,
                    usize::try_from(span.end_offset).map_err(|_| {
                        AppError::InvalidArgument(format!(
                            "Galgame source span {} 终点无效",
                            span.id
                        ))
                    })?,
                    span.text.clone(),
                ))
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let marked_content = source_markers.mark_ranges(&chapter.content, ranges)?;
        marked_chapter["content"] = Value::String(marked_content);
    }
    Ok(marked)
}

pub fn galgame_prompt_chapter_from_snapshot(
    chapter: &GalgamePipelinePromptChapterSourceRecord,
    anchors: &[GalgamePipelinePromptTextAnchorRecord],
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
                    chapter_number: chapter.chapter_number,
                })
        })
        .collect()
}

pub fn galgame_speaker_profiles_from_prompt_records(
    profiles: &[GalgamePipelinePromptSpeakerProfileRecord],
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
