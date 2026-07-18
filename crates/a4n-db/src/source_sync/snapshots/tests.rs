use super::scope::parse_source_sync_chapter_scope;
use crate::project::DerivedChapterScope;
use crate::source_sync::SourceSyncDbError;
use serde_json::json;

#[test]
fn parse_source_sync_chapter_scope_accepts_valid_range() {
    let scope = parse_source_sync_chapter_scope(&json!({"start": 2, "end": 5}))
        .expect("valid chapter scope");

    assert_eq!(
        scope,
        DerivedChapterScope {
            start_chapter: 2,
            end_chapter: 5
        }
    );
}

#[test]
fn parse_source_sync_chapter_scope_rejects_missing_start() {
    let err = parse_source_sync_chapter_scope(&json!({"end": 5}))
        .expect_err("missing start must be rejected");

    assert!(matches!(
        err,
        SourceSyncDbError::InvalidArgument(message)
            if message == "chapter_scope_json.start 缺失"
    ));
}

#[test]
fn parse_source_sync_chapter_scope_rejects_inverted_range() {
    let err = parse_source_sync_chapter_scope(&json!({"start": 6, "end": 5}))
        .expect_err("inverted range must be rejected");

    assert!(matches!(
        err,
        SourceSyncDbError::InvalidArgument(message)
            if message == "chapter_scope_json 范围无效"
    ));
}
