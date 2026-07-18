use super::super::character_discovery::existing_profile_for_prompt;
use super::super::*;
use super::fixtures::discovery_chapter;
use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::source_spans::source_spans_for_chapters;
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn discovery_anchor_context(content: &str) -> (Vec<chapter::Model>, Vec<text_anchor::Model>) {
    let chapters = vec![discovery_chapter(1, "draft", content)];
    (chapters.clone(), anchors_for_test(&chapters))
}

fn anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .iter()
        .enumerate()
        .map(|(index, span)| text_anchor::Model {
            id: i64::try_from(index + 1).unwrap(),
            project_id: 1,
            source_chapter_id: Some(span.chapter_id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: span.chapter_id,
            paragraph_index: i32::try_from(span.sentence_index).unwrap(),
            start_offset: i32::try_from(span.char_start).unwrap(),
            end_offset: i32::try_from(span.char_end).unwrap(),
            quote: span.text.clone(),
            quote_hash: format!("hash-{}", index + 1),
            source_version_hash: "source-version".to_string(),
            created_at: now,
        })
        .collect()
}

fn anchor_id_containing(anchors: &[text_anchor::Model], text: &str) -> i64 {
    anchors
        .iter()
        .find(|anchor| anchor.quote.contains(text))
        .map(|anchor| anchor.id)
        .unwrap_or_else(|| panic!("missing text anchor containing {text}"))
}
