use super::super::*;
use super::fixtures::{discovery_chapter, text_anchors_for_test};
use a4n_core::source_spans::text_anchor_prompt_values;
use serde_json::{Value, json};

fn chapter_with_text_anchors(chapter_number: i32, content: &str) -> Value {
    let chapter = discovery_chapter(chapter_number, "completed", content);
    let anchors = text_anchors_for_test(std::slice::from_ref(&chapter));
    json!({
        "chapter_number": chapter_number,
        "content": content,
        "text_anchors": text_anchor_prompt_values(&[chapter], &anchors)
    })
}

fn anchor_id_containing(chapter: &Value, needle: &str) -> i64 {
    chapter["text_anchors"]
        .as_array()
        .expect("text_anchors should be array")
        .iter()
        .find(|anchor| {
            anchor["quote"]
                .as_str()
                .is_some_and(|text| text.contains(needle))
        })
        .and_then(|anchor| anchor["id"].as_i64())
        .expect("text anchor should exist")
}

#[test]
fn validate_sync_item_chapters_rejects_chapters_not_in_sync_input() {
    let items = vec![json!({
        "chapter_number": 9,
        "category": "explicit",
        "key": "位置",
        "operation": "modify",
        "new_value": "新地",
        "change_description": "越界变化",
        "event_cause": "错误章节",
        "evidence_anchor_id": 9
    })];

    let err = validate_sync_item_chapters("attribute_changes", &items, &[10, 12])
        .expect_err("chapter missing from sync input must fail");
    assert!(err.contains("不在本次同步输入章节中"));
    assert!(err.contains("第 10 章、第 12 章"));
}

#[test]
fn validate_sync_evidence_materializes_anchor_ids_from_matching_chapter() {
    let chapter = chapter_with_text_anchors(1, "雨声渐密，他沉默片刻后才开口。");
    let anchor_id = anchor_id_containing(&chapter, "他沉默片刻后才开口");
    let mut changes = vec![json!({
        "chapter_number": 1,
        "evidence_anchor_id": anchor_id
    })];
    let mut behaviors = vec![json!({
        "chapter_number": 1,
        "original_text_anchor_id": anchor_id
    })];
    let mut deletion_candidates = vec![json!({
        "chapter_number": 1,
        "evidence_anchor_id": anchor_id
    })];

    validate_sync_evidence_against_chapters(
        &mut changes,
        &mut behaviors,
        &mut deletion_candidates,
        &[chapter],
    )
    .expect("anchor evidence from matching chapter should pass");

    assert_eq!(changes[0]["evidence"], json!("他沉默片刻后才开口"));
    assert!(changes[0].get("evidence_anchor_id").is_none());
    assert_eq!(behaviors[0]["original_text"], json!("他沉默片刻后才开口"));
    assert!(behaviors[0].get("original_text_anchor_id").is_none());
    assert_eq!(
        deletion_candidates[0]["evidence"],
        json!("他沉默片刻后才开口")
    );
}

#[test]
fn validate_sync_evidence_rejects_legacy_free_text_evidence() {
    let chapter = chapter_with_text_anchors(1, "陈恪伸手摸了摸额头，果然指尖触到一块粗糙的结痂。");
    let mut changes = vec![json!({
        "chapter_number": 1,
        "evidence": "他伸手摸了摸额头，果然指尖触到一块粗糙的结痂"
    })];

    let err = validate_sync_evidence_against_chapters(&mut changes, &mut [], &mut [], &[chapter])
        .expect_err("legacy free text evidence should fail");
    assert!(err.contains("evidence_anchor_id"));
}

#[test]
fn validate_sync_evidence_rejects_summary_without_anchor_id() {
    let chapter = chapter_with_text_anchors(
        1,
        "小宦官低声道：“今日是武德九年六月初三。”陈恪心头一紧，明日便是玄武门之变。",
    );
    let mut changes = vec![json!({
        "chapter_number": 1,
        "evidence": "陈恪从宦官口中得知今日是武德九年六月初三，意识到明天就是玄武门之变"
    })];

    let err = validate_sync_evidence_against_chapters(&mut changes, &mut [], &mut [], &[chapter])
        .expect_err("summary evidence without anchor id should fail");
    assert!(err.contains("attribute_changes[0].evidence_anchor_id"));
}

#[test]
fn validate_sync_evidence_rejects_unknown_anchor_id() {
    let chapter = chapter_with_text_anchors(1, "他沉默片刻后才开口。");
    let mut changes = vec![json!({
        "chapter_number": 1,
        "evidence_anchor_id": 999
    })];

    let err = validate_sync_evidence_against_chapters(&mut changes, &mut [], &mut [], &[chapter])
        .expect_err("unknown text anchor id should fail");

    assert!(err.contains("attribute_changes[0].evidence_anchor_id"));
    assert!(err.contains("text_anchors"));
}

#[test]
fn validate_sync_evidence_rejects_missing_text_anchors() {
    let mut changes = vec![json!({
        "chapter_number": 1,
        "evidence_anchor_id": 1
    })];
    let chapters = vec![json!({
        "chapter_number": 1,
        "content": "他沉默片刻后才开口。"
    })];

    let err = validate_sync_evidence_against_chapters(&mut changes, &mut [], &mut [], &chapters)
        .expect_err("chapter without text_anchors should fail");

    assert!(err.contains("缺少 text_anchors"));
}
