use a4n_db::galgame::{
    GalgamePipelinePromptChapterSourceRecord, GalgamePipelinePromptTextAnchorRecord,
};
use serde_json::json;

use super::*;
use crate::galgame::GalgameSpeakerProfile;

#[test]
fn builds_prompt_chapter_from_snapshot_anchors() {
    let chapter = snapshot_chapter(20, 1, "雨夜车站。她回过头。");
    let anchors = vec![
        text_anchor(101, 20, 2, 5, 9, "她回过头"),
        text_anchor(100, 20, 1, 0, 4, "雨夜车站"),
    ];

    let prompt_chapter =
        galgame_prompt_chapter_from_snapshot(&chapter, &anchors).expect("prompt chapter");

    assert_eq!(prompt_chapter.source_spans.len(), 2);
    assert_eq!(prompt_chapter.source_spans[0].id, "ch1:ta100");
    assert_eq!(prompt_chapter.source_spans[1].text_anchor_id, 101);
}

#[test]
fn prompt_chapter_rejects_missing_snapshot_anchors_without_content_fallback() {
    let chapter = snapshot_chapter(20, 1, "雨夜车站。她回过头。");
    let anchors = vec![text_anchor(100, 21, 1, 0, 5, "其他章节")];

    let err = galgame_prompt_chapter_from_snapshot(&chapter, &anchors)
        .expect_err("Galgame prompt chapter must require same-chapter text anchors");

    assert!(
        err.to_string()
            .contains("source_snapshot_chapter_20_has_no_prompt_spans")
    );
    assert!(
        err.to_string()
            .contains("build_galgame_pipeline_prompt_chapter")
    );
}

#[test]
fn builds_persist_source_spans_from_prompt_chapters() {
    let chapter = galgame_prompt_chapter_from_snapshot(
        &snapshot_chapter(20, 1, "雨夜车站。"),
        &[text_anchor(100, 20, 1, 0, 4, "雨夜车站")],
    )
    .expect("prompt chapter");

    let spans = galgame_pipeline_source_spans_from_prompt_chapters(&[chapter]);

    assert_eq!(spans[0].id, "ch1:ta100");
    assert_eq!(spans[0].text_anchor_id, 100);
    assert_eq!(spans[0].source_snapshot_chapter_id, 20);
}

#[test]
fn character_staging_message_includes_speaker_profiles() {
    let source_markers = galgame_source_markers(&[]).expect("source markers");
    let message = galgame_character_staging_user_message(
        &json!({"scene_lines": []}),
        &[GalgameSpeakerProfile {
            id: 7,
            name: "林晓雨".to_string(),
        }],
        &source_markers,
    )
    .expect("message");

    assert!(message.contains("\"speaker_profiles\""));
    assert!(message.contains("\"林晓雨\""));
}

#[test]
fn script_adaptation_uses_short_markers_without_colliding_with_source_text() {
    let chapter = galgame_prompt_chapter_from_snapshot(
        &snapshot_chapter(20, 1, "#0开头。"),
        &[text_anchor(100, 20, 1, 0, 4, "#0开头")],
    )
    .expect("prompt chapter");
    let source_markers =
        galgame_source_markers(std::slice::from_ref(&chapter)).expect("source markers");

    assert_eq!(source_markers.marker_for("ch1:ta100").unwrap(), "#1");
    let message = galgame_script_adaptation_user_message(
        std::slice::from_ref(&chapter),
        &json!({"scenes": [{"source_span_ids": ["ch1:ta100"]}]}),
        &json!({}),
        &source_markers,
    )
    .expect("marked script adaptation message");
    let payload: serde_json::Value =
        serde_json::from_str(a4n_prompt::request::json_user_message_payload(&message).unwrap())
            .unwrap();

    assert_eq!(payload["input"]["chapters"][0]["content"], "#1 #0开头。");
    assert!(
        payload["input"]["chapters"][0]
            .get("source_spans")
            .is_none()
    );
    assert_eq!(
        payload["input"]["previous_steps"]["adaptation_projection"]["scenes"][0]["source_span_ids"],
        json!(["#1"])
    );
}

fn snapshot_chapter(
    id: i64,
    chapter_number: i32,
    content: &str,
) -> GalgamePipelinePromptChapterSourceRecord {
    GalgamePipelinePromptChapterSourceRecord {
        id,
        source_chapter_id: 5,
        chapter_number,
        title: Some("测试章节".to_string()),
        content: content.to_string(),
        summary: Some("章节摘要".to_string()),
        content_hash: "hash".to_string(),
    }
}

fn text_anchor(
    id: i64,
    source_snapshot_chapter_id: i64,
    paragraph_index: i32,
    start_offset: i32,
    end_offset: i32,
    quote: &str,
) -> GalgamePipelinePromptTextAnchorRecord {
    GalgamePipelinePromptTextAnchorRecord {
        id,
        source_snapshot_chapter_id: Some(source_snapshot_chapter_id),
        paragraph_index,
        start_offset,
        end_offset,
        quote: quote.to_string(),
    }
}
