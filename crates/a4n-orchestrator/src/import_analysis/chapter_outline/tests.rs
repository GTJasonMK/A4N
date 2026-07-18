use super::*;
use a4n_core::models::chapter::chapter;
use chrono::NaiveDate;
use serde_json::{Value, json};

pub(super) fn sample_chapter() -> chapter::Model {
    let now = NaiveDate::from_ymd_opt(2026, 5, 26)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    chapter::Model {
        id: 11,
        project_id: 7,
        outline_id: None,
        chapter_number: 3,
        title: Some("旧城回声".to_string()),
        content: "林岚在旧城发现星潮线索。".to_string(),
        word_count: 13,
        status: "imported".to_string(),
        summary: None,
        style_tags: None,
        analysis_data: Some(sample_analysis()),
        created_at: now,
        updated_at: now,
    }
}

fn sample_analysis() -> Value {
    json!({
        "metadata": {
            "characters": ["林岚"],
            "locations": ["旧城"],
            "items": ["星潮残片"],
            "tags": ["转折"]
        },
        "summaries": {
            "compressed": "林岚在旧城发现星潮线索。",
            "keywords": ["旧城", "星潮"]
        },
        "key_events": ["发现星潮线索"],
        "foreshadowing": {
            "planted": ["残片发光"],
            "resolved": []
        }
    })
}

#[test]
fn required_chapter_title_rejects_missing_value() {
    let mut chapter = sample_chapter();
    chapter.title = None;

    let err = required_chapter_title(&chapter).expect_err("missing title must fail");

    assert!(err.contains("缺少章节标题"));
}

#[test]
fn chapter_outline_summary_rejects_missing_summary() {
    let mut chapter = sample_chapter();
    chapter.analysis_data = None;
    chapter.summary = None;

    let err = chapter_outline_summary(&chapter).expect_err("missing summary must fail");

    assert!(err.contains("缺少结构化摘要"));
}

#[test]
fn chapter_outline_summary_rejects_generated_word_count_summary() {
    let mut chapter = sample_chapter();
    chapter.analysis_data = None;
    chapter.summary = Some("第3章：13字".to_string());

    let err =
        chapter_outline_summary(&chapter).expect_err("generated word-count summary must fail");

    assert!(err.contains("缺少结构化摘要"));
}

#[test]
fn chapter_outline_summary_accepts_analysis_summary() {
    let chapter = sample_chapter();

    assert_eq!(
        chapter_outline_summary(&chapter).unwrap(),
        "林岚在旧城发现星潮线索。"
    );
}

#[test]
fn collect_chapter_summaries_rejects_missing_title() {
    let mut chapter = sample_chapter();
    chapter.title = Some("   ".to_string());

    let err = collect_chapter_summaries(&[chapter], 80).expect_err("blank title must fail");

    assert!(err.contains("缺少章节标题"));
}

#[test]
fn analysis_value_path_rejects_non_array_leaf() {
    let analysis = json!({
        "key_events": "发现星潮线索"
    });

    let err = analysis_value_path(Some(&analysis), &["key_events"], 3)
        .expect_err("non-array key_events must fail");

    assert!(err.contains("key_events 必须是数组"));
}

#[test]
fn analysis_metadata_array_rejects_non_array_leaf() {
    let analysis = json!({
        "metadata": {
            "characters": "林岚"
        }
    });

    let err = analysis_metadata_array(Some(&analysis), "characters", 3)
        .expect_err("non-array metadata.characters must fail");

    assert!(err.contains("metadata.characters 必须是数组"));
}

#[test]
fn collect_part_array_values_preserves_items_until_limit() {
    let chapter = sample_chapter();
    let values = collect_part_array_values(&[&chapter], 1, |chapter| {
        analysis_value_path(
            chapter.analysis_data.as_ref(),
            &["key_events"],
            chapter.chapter_number,
        )
    })
    .expect("valid key_events should collect");

    assert_eq!(values, vec![json!("发现星潮线索")]);
}
