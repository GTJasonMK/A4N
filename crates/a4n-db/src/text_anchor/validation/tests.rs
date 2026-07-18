use a4n_core::models::project::text_anchor;
use chrono::NaiveDateTime;

use super::rules::{normalize_anchor_ids, validate_loaded_anchor_records};
use crate::text_anchor::TextAnchorDbError;

fn anchor(
    id: i64,
    project_id: i64,
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
) -> text_anchor::Model {
    text_anchor::Model {
        id,
        project_id,
        source_chapter_id: None,
        source_snapshot_id,
        source_snapshot_chapter_id,
        source_kind: "chapter_content".to_string(),
        source_id: 1,
        paragraph_index: 0,
        start_offset: 0,
        end_offset: 1,
        quote: "锚".to_string(),
        quote_hash: "quote-hash".to_string(),
        source_version_hash: "source-hash".to_string(),
        created_at: NaiveDateTime::MIN,
    }
}

#[test]
fn normalize_anchor_ids_rejects_empty_and_non_positive_ids() {
    assert!(matches!(
        normalize_anchor_ids(&[]),
        Err(TextAnchorDbError::InvalidArgument(message))
            if message == "text_anchor_ids 不能为空"
    ));
    assert!(matches!(
        normalize_anchor_ids(&[1, 0]),
        Err(TextAnchorDbError::InvalidArgument(message))
            if message == "text_anchor_id 必须为正整数: 0"
    ));
}

#[test]
fn normalize_anchor_ids_stably_deduplicates_ids() {
    let ids = normalize_anchor_ids(&[8, 3, 8]).expect("valid anchor ids must normalize");

    assert_eq!(ids.into_iter().collect::<Vec<_>>(), vec![3, 8]);
}

#[test]
fn validate_loaded_anchor_records_rejects_missing_and_wrong_snapshot() {
    let anchors = vec![anchor(3, 7, Some(20), Some(30))];
    assert!(matches!(
        validate_loaded_anchor_records(&anchors, 2, 7, Some(20), &[30]),
        Err(TextAnchorDbError::InvalidArgument(message))
            if message == "存在不存在的 text_anchor_id"
    ));
    assert!(matches!(
        validate_loaded_anchor_records(&anchors, 1, 7, Some(21), &[30]),
        Err(TextAnchorDbError::InvalidArgument(message))
            if message == "text_anchor_id=3 不属于目标源快照"
    ));
}

#[test]
fn validate_loaded_anchor_records_enforces_snapshot_chapter_scope() {
    let anchors = vec![anchor(3, 7, Some(20), Some(30))];
    assert!(matches!(
        validate_loaded_anchor_records(&anchors, 1, 7, Some(20), &[31]),
        Err(TextAnchorDbError::InvalidArgument(message))
            if message == "text_anchor_id=3 不属于目标章节范围"
    ));
    assert!(validate_loaded_anchor_records(&anchors, 1, 7, Some(20), &[30]).is_ok());
}
