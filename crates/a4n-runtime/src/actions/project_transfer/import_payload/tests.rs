use super::*;
use serde_json::{Value, json};

fn part(part_number: i32, start_chapter: i32, end_chapter: i32) -> Value {
    json!({
        "part_number": part_number,
        "title": format!("第{part_number}部"),
        "start_chapter": start_chapter,
        "end_chapter": end_chapter,
    })
}

fn outline(id: i64, chapter_number: i32) -> Value {
    json!({
        "id": id,
        "chapter_number": chapter_number,
    })
}

fn chapter_with_outline(chapter_number: i32, outline_id: i64) -> Value {
    json!({
        "chapter_number": chapter_number,
        "outline_id": outline_id,
        "content": "旧城A",
        "word_count": 3,
    })
}

fn part_with_id(id: i64, part_number: i32, start_chapter: i32, end_chapter: i32) -> Value {
    let mut value = part(part_number, start_chapter, end_chapter);
    value["id"] = json!(id);
    value
}

#[test]
fn parse_import_payload_rejects_overlapping_part_ranges() {
    let payload = json!({
        "chapters": [],
        "part_outlines": [
            part(1, 1, 10),
            part(2, 10, 20)
        ]
    });

    let err = parse_import_payload(&payload).expect_err("overlapping part ranges must be rejected");

    assert!(err.contains("分部章节范围重叠"));
    assert!(err.contains("第1部(1-10)"));
    assert!(err.contains("第2部(10-20)"));
}

#[test]
fn parse_import_payload_accepts_adjacent_part_ranges() {
    let payload = json!({
        "chapters": [],
        "part_outlines": [
            part(1, 1, 10),
            part(2, 11, 20)
        ]
    });

    let parsed = parse_import_payload(&payload).expect("adjacent part ranges should be valid");

    assert_eq!(parsed.part_outlines.len(), 2);
}

#[test]
fn parse_import_payload_rejects_chapter_outline_reference_for_other_chapter() {
    let payload = json!({
        "chapters": [
            chapter_with_outline(1, 102)
        ],
        "chapter_outlines": [
            outline(102, 2)
        ]
    });

    let err = parse_import_payload(&payload)
        .expect_err("chapter content must not point at another chapter outline");

    assert!(err.contains("第1章引用的章节大纲属于第2章"));
    assert!(err.contains("outline_id=102"));
}

#[test]
fn parse_import_payload_accepts_same_chapter_outline_reference() {
    let payload = json!({
        "chapters": [
            chapter_with_outline(1, 101)
        ],
        "chapter_outlines": [
            outline(101, 1)
        ]
    });

    let parsed =
        parse_import_payload(&payload).expect("same-chapter outline reference should import");

    assert_eq!(parsed.chapter_outlines.len(), 1);
}

#[test]
fn parse_import_payload_rejects_chapter_outline_part_outside_range() {
    let payload = json!({
        "chapters": [],
        "part_outlines": [
            part_with_id(101, 1, 1, 10)
        ],
        "chapter_outlines": [
            { "id": 201, "part_id": 101, "chapter_number": 11 }
        ]
    });

    let err = parse_import_payload(&payload)
        .expect_err("chapter outline part_id must contain the chapter number");

    assert!(err.contains("第11章大纲引用的第1部分部范围不包含该章节"));
    assert!(err.contains("part_id=101"));
    assert!(err.contains("range=1-10"));
}

#[test]
fn parse_import_payload_accepts_chapter_outline_part_inside_range() {
    let payload = json!({
        "chapters": [],
        "part_outlines": [
            part_with_id(101, 1, 1, 10)
        ],
        "chapter_outlines": [
            { "id": 201, "part_id": 101, "chapter_number": 10 }
        ]
    });

    let parsed = parse_import_payload(&payload)
        .expect("chapter outline may reference a part that contains the chapter");

    assert_eq!(parsed.chapter_outlines.len(), 1);
}

#[test]
fn parse_import_payload_rejects_unknown_chapter_outline_status() {
    let payload = json!({
        "chapters": [],
        "chapter_outlines": [
            { "id": 201, "chapter_number": 1, "status": "bogus" }
        ]
    });

    let err = parse_import_payload(&payload)
        .expect_err("chapter outline status must use known persistent states");

    assert!(err.contains("导入章节大纲1状态无效: bogus"));
}

#[test]
fn parse_import_payload_accepts_known_chapter_outline_statuses() {
    for status in [
        chapter_outline::STATUS_DRAFT,
        chapter_outline::STATUS_COMPLETED,
    ] {
        let payload = json!({
            "chapters": [],
            "chapter_outlines": [
                { "id": 201, "chapter_number": 1, "status": status }
            ]
        });

        let parsed =
            parse_import_payload(&payload).expect("known chapter outline status should import");

        assert_eq!(parsed.chapter_outlines[0].status, status);
    }
}

#[test]
fn parse_import_payload_defaults_missing_chapter_outline_status_to_draft() {
    let payload = json!({
        "chapters": [],
        "chapter_outlines": [
            { "id": 201, "chapter_number": 1 }
        ]
    });

    let parsed = parse_import_payload(&payload)
        .expect("missing chapter outline status should default to draft");

    assert_eq!(
        parsed.chapter_outlines[0].status,
        chapter_outline::STATUS_DRAFT
    );
}

#[test]
fn parse_import_payload_rejects_chapter_word_count_mismatch() {
    let payload = json!({
        "chapters": [
            { "chapter_number": 1, "content": "旧城A", "word_count": 2 }
        ]
    });

    let err = parse_import_payload(&payload)
        .expect_err("declared word count must match imported content characters");

    assert!(err.contains("第1章字数与正文字符数不一致"));
    assert!(err.contains("word_count=2"));
    assert!(err.contains("actual=3"));
}

#[test]
fn import_chapter_status_defaults_missing_status_to_imported() {
    let status = import_chapter_status(&json!({}), "第1章")
        .expect("missing chapter status should use imported");

    assert_eq!(status, chapter::STATUS_IMPORTED);
}

#[test]
fn import_chapter_status_preserves_known_persistent_statuses() {
    for status in [
        chapter::STATUS_DRAFT,
        chapter::STATUS_IMPORTED,
        chapter::STATUS_COMPLETED,
        chapter::STATUS_REVIEWED,
        chapter::STATUS_PUBLISHED,
        chapter::STATUS_DELETED,
    ] {
        let parsed = import_chapter_status(&json!({ "status": status }), "第1章")
            .expect("known persistent status should remain importable");

        assert_eq!(parsed, status);
    }
}

#[test]
fn import_chapter_status_rejects_unknown_status() {
    let err = import_chapter_status(&json!({ "status": "bogus" }), "第1章")
        .expect_err("unknown chapter status must fail");

    assert!(err.contains("第1章状态无效: bogus"));
}

#[test]
fn import_chapter_status_rejects_generating_status() {
    let err = import_chapter_status(&json!({ "status": "generating" }), "第1章")
        .expect_err("import must not create a stuck generating chapter");

    assert!(err.contains("生成中的瞬态状态"));
    assert!(err.contains("generating"));
}
