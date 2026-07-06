use super::super::record_builders::{outline_rag_record, outline_rag_text, part_outline_rag_text};
use super::{sample_outline, sample_part};
use a4n_core::rag::RagDataType;
use serde_json::json;

#[test]
fn outline_rag_text_requires_title() {
    let mut outline = sample_outline();
    outline.title = Some("  ".to_string());

    let err = outline_rag_text(&outline).expect_err("blank title should reject");

    assert!(err.contains("缺少标题"));
}

#[test]
fn outline_rag_text_requires_summary() {
    let mut outline = sample_outline();
    outline.summary = None;

    let err = outline_rag_text(&outline).expect_err("missing summary should reject");

    assert!(err.contains("缺少摘要"));
}

#[test]
fn outline_rag_text_includes_real_fields() {
    let text = outline_rag_text(&sample_outline()).expect("valid outline text");

    assert!(text.contains("标题: 入城"));
    assert!(text.contains("摘要: 主角进入边境城。"));
    assert!(text.contains("节拍: 1. 抵达"));
    assert!(!text.contains("无标题"));
}

#[test]
fn outline_rag_record_uses_canonical_id_text_and_metadata() {
    let record = outline_rag_record(9, &sample_outline()).expect("valid outline record");

    assert_eq!(record.chunk_id, "outline:9:10");
    assert!(record.text.contains("标题: 入城"));
    assert_eq!(
        record.metadata["data_type"],
        json!(RagDataType::Outline.as_str())
    );
    assert_eq!(record.metadata["outline_id"], json!(10));
    assert_eq!(record.metadata["chapter_number"], json!(3));
}

#[test]
fn part_outline_rag_text_requires_summary() {
    let mut part = sample_part();
    part.summary = Some(" ".to_string());

    let err = part_outline_rag_text(&part).expect_err("blank summary should reject");

    assert!(err.contains("缺少摘要"));
}

#[test]
fn part_outline_rag_text_includes_range_and_optional_fields() {
    let text = part_outline_rag_text(&sample_part()).expect("valid part text");

    assert!(text.contains("分部: 第一部"));
    assert!(text.contains("章节范围: 第1章-第8章"));
    assert!(text.contains("主题: 选择"));
}
