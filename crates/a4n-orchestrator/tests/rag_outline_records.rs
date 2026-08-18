use a4n_core::{
    models::chapter::{chapter_outline, part_outline},
    rag::RagDataType,
};
use a4n_orchestrator::rag::{outline_rag_record, part_outline_rag_record};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::json;

fn ts() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid date")
        .and_hms_opt(0, 0, 0)
        .expect("valid time")
}

fn sample_outline() -> chapter_outline::Model {
    chapter_outline::Model {
        id: 10,
        project_id: 1,
        part_id: Some(2),
        chapter_number: 3,
        title: Some("入城".to_string()),
        summary: Some("主角进入边境城。".to_string()),
        beats: Some(json!(["抵达", "发现异常"])),
        key_scenes: Some(json!([{"title": "城门", "description": "守卫盘查"}])),
        pacing_score: Some(0.8),
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: Some(3000),
        status: "planned".to_string(),
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_part() -> part_outline::Model {
    part_outline::Model {
        id: 20,
        project_id: 1,
        part_number: 1,
        title: "第一部".to_string(),
        start_chapter: 1,
        end_chapter: 8,
        summary: Some("主角建立目标并进入冲突。".to_string()),
        theme: Some("选择".to_string()),
        key_events: Some(json!(["离乡", "结盟"])),
        character_focus: Some(json!(["林舟"])),
        created_at: ts(),
        updated_at: ts(),
    }
}

#[test]
fn outline_record_rejects_missing_required_text() {
    let mut outline = sample_outline();
    outline.title = Some("  ".to_string());
    assert!(outline_rag_record(9, &outline).is_err());

    outline = sample_outline();
    outline.summary = None;
    assert!(outline_rag_record(9, &outline).is_err());
}

#[test]
fn outline_record_preserves_text_and_metadata() {
    let outline = sample_outline();
    let record = outline_rag_record(9, &outline).expect("outline record");

    assert_eq!(record.chunk_id, "outline:9:10");
    assert!(record.text.contains("标题: 入城"));
    assert!(record.text.contains("摘要: 主角进入边境城。"));
    assert!(record.text.contains("节拍: 1. 抵达"));
    assert_eq!(record.metadata["data_type"], RagDataType::Outline.as_str());
    assert_eq!(record.metadata["chapter_number"], 3);
}

#[test]
fn part_outline_record_rejects_missing_required_text() {
    let mut part = sample_part();
    part.title = " ".to_string();
    assert!(part_outline_rag_record(9, &part).is_err());

    part = sample_part();
    part.summary = Some(" ".to_string());
    assert!(part_outline_rag_record(9, &part).is_err());
}

#[test]
fn part_outline_record_preserves_range_optional_fields_and_metadata() {
    let part = sample_part();
    let record = part_outline_rag_record(9, &part).expect("part outline record");

    assert_eq!(record.chunk_id, "part_outline:9:20");
    assert!(record.text.contains("分部: 第一部"));
    assert!(record.text.contains("章节范围: 第1章-第8章"));
    assert!(record.text.contains("主题: 选择"));
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::PartOutline.as_str()
    );
}
