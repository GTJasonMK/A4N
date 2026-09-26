use std::collections::HashSet;

use a4n_domain::source_spans::SourceSpan;

use super::super::{AppError, VideoCinematicShot, VideoSegmentPlan};

pub(in crate::video::prompts::generation::frame_prompts) fn source_spans_for_shot(
    source_spans: &[SourceSpan],
    shot: &VideoCinematicShot,
) -> Result<Vec<SourceSpan>, AppError> {
    let ids = shot
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    select_source_spans(source_spans, &ids)
}

pub(in crate::video::prompts::generation::frame_prompts) fn source_spans_for_segments<'a>(
    source_spans: &[SourceSpan],
    segments: impl Iterator<Item = &'a VideoSegmentPlan>,
) -> Result<Vec<SourceSpan>, AppError> {
    let ids = segments
        .flat_map(|segment| segment.source_span_ids.iter().map(String::as_str))
        .collect();
    select_source_spans(source_spans, &ids)
}

fn select_source_spans(
    source_spans: &[SourceSpan],
    ids: &HashSet<&str>,
) -> Result<Vec<SourceSpan>, AppError> {
    let selected = source_spans
        .iter()
        .filter(|span| ids.contains(span.id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if selected.len() != ids.len() {
        return Err(AppError::InvalidArgument(
            "视频输入引用了不存在的原文片段".into(),
        ));
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_selection_rejects_a_partially_missing_scope() {
        let spans = a4n_domain::source_spans::source_spans_for_content(1, 1, "打开门。走进屋。");
        let valid = HashSet::from([spans[0].id.as_str()]);
        assert_eq!(select_source_spans(&spans, &valid).unwrap().len(), 1);
        let invalid = HashSet::from([spans[0].id.as_str(), "missing"]);
        assert!(select_source_spans(&spans, &invalid).is_err());
    }
}
