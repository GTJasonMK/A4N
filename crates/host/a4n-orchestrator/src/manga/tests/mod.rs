use super::pipeline::prompt::{
    build_manga_step_user_message, manga_item_available_source_span_ids,
    manga_item_extraction_input, manga_page_drawing_prompt_page_input, manga_page_planning_input,
    manga_speech_selection_user_message, manga_visual_anchor_extraction_user_message,
    validate_manga_character_binding_source_span_ids,
    validate_manga_item_extraction_source_span_ids,
    validate_manga_speech_selection_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
use a4n_domain::source_spans::source_spans_for_content;
use serde_json::json;

mod context;
mod messages;
mod validation;

pub(super) fn known_beats(
    spans: &[a4n_domain::source_spans::SourceSpan],
    count: u32,
) -> super::adaptation_projection::MangaBeatIndex {
    use a4n_domain::media_authoring::{
        AdaptationBeat, AdaptationBeatKind, AdaptationBeatRef, AdaptationImportance,
        AdaptationOrigin,
    };
    (1..=count)
        .map(|number| {
            let id = format!("beat-{number}");
            (
                AdaptationBeatRef {
                    unit_id: "unit-1".into(),
                    beat_id: id.clone(),
                },
                AdaptationBeat {
                    id,
                    kind: AdaptationBeatKind::Action,
                    is_climax: false,
                    origin: if spans.is_empty() {
                        AdaptationOrigin::Historical
                    } else {
                        AdaptationOrigin::Source
                    },
                    importance: AdaptationImportance::Required,
                    action: "呈现原文事件".into(),
                    reaction: None,
                    dialogue_intent: None,
                    entry_state: "行动前".into(),
                    exit_state: "行动后".into(),
                    source_span_ids: spans.iter().map(|span| span.id.clone()).collect(),
                },
            )
        })
        .collect()
}
