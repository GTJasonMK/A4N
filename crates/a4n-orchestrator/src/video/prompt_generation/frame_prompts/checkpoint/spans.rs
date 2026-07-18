use std::collections::HashSet;

use a4n_core::source_spans::SourceSpan;

use super::super::VideoCinematicShot;

pub(in crate::video::prompt_generation::frame_prompts) fn source_spans_for_shot(
    source_spans: &[SourceSpan],
    shot: &VideoCinematicShot,
) -> Vec<SourceSpan> {
    let ids = shot
        .source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    source_spans
        .iter()
        .filter(|span| ids.contains(span.id.as_str()))
        .cloned()
        .collect()
}
