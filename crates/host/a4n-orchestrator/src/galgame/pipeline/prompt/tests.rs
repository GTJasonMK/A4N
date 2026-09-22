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
        text_anchor(101, 20, 2, 6, 11, "她回过头"),
        text_anchor(100, 20, 1, 0, 5, "雨夜车站"),
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
        &[text_anchor(100, 20, 1, 0, 5, "雨夜车站")],
    )
    .expect("prompt chapter");

    let spans = galgame_pipeline_source_spans_from_prompt_chapters(&[chapter]);

    assert_eq!(spans[0].id, "ch1:ta100");
    assert_eq!(spans[0].text_anchor_id, 100);
    assert_eq!(spans[0].source_snapshot_chapter_id, 20);
}

#[test]
fn character_staging_message_includes_speaker_profiles() {
    let message = galgame_character_staging_user_message(
        &json!({"scene_lines": []}),
        &[GalgameSpeakerProfile {
            id: 7,
            name: "林晓雨".to_string(),
        }],
    )
    .expect("message");

    assert!(message.contains("\"speaker_profiles\""));
    assert!(message.contains("\"林晓雨\""));
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
