use serde_json::json;

use super::validate_reverse_outline_items;

#[test]
fn rejects_non_positive_chapter_number() {
    let outlines = vec![json!({
        "chapter_number": 0,
        "title": "第零章",
        "summary": "非法章节"
    })];

    let err = validate_reverse_outline_items(&outlines)
        .expect_err("chapter number zero must fail before persistence");

    assert!(err.contains("chapter_number 必须大于 0"));
}

#[test]
fn rejects_unknown_fields() {
    let outlines = vec![json!({
        "chapter_number": 1,
        "title": "第一章",
        "summary": "章节摘要",
        "legacy_notes": "旧字段"
    })];

    let err =
        validate_reverse_outline_items(&outlines).expect_err("unknown outline fields must fail");

    assert!(err.contains("未知字段 legacy_notes"));
}

#[test]
fn rejects_general_outline_fields() {
    let outlines = vec![json!({
        "chapter_number": 1,
        "title": "第一章",
        "summary": "章节摘要",
        "beats": ["旧节拍"]
    })];

    let err = validate_reverse_outline_items(&outlines)
        .expect_err("reverse outline must not accept general outline fields");

    assert!(err.contains("未知字段 beats"));
}
