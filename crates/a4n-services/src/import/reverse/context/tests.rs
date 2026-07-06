use a4n_core::models::chapter::chapter;
use chrono::NaiveDate;
use serde_json::{Value, json};

use super::chapter_entries::{chapter_summary_for_reverse, sample_chapter_entry};
use super::extraction::{collect_extracted_characters, dedup_character_records};

fn sample_chapter(analysis_data: Option<Value>) -> chapter::Model {
    sample_chapter_with_number(2, analysis_data)
}

fn sample_chapter_with_number(chapter_number: i32, analysis_data: Option<Value>) -> chapter::Model {
    let now = NaiveDate::from_ymd_opt(2026, 5, 26)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    chapter::Model {
        id: 9,
        project_id: 3,
        outline_id: None,
        chapter_number,
        title: Some("第二章".into()),
        content: "第一段\n\n第二段".into(),
        word_count: 6,
        status: "completed".into(),
        summary: None,
        style_tags: None,
        analysis_data,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn sample_chapter_entry_omits_missing_analysis() {
    let chapter = sample_chapter(None);

    let entry = sample_chapter_entry(&chapter).unwrap();

    assert_eq!(entry["chapter_number"], 2);
    assert_eq!(entry["title"], "第二章");
    assert!(entry.get("analysis").is_none());
}

#[test]
fn sample_chapter_entry_omits_missing_title() {
    let mut chapter = sample_chapter(None);
    chapter.title = None;

    let entry = sample_chapter_entry(&chapter).unwrap();

    assert_eq!(entry["chapter_number"], 2);
    assert!(entry.get("title").is_none());
}

#[test]
fn sample_chapter_entry_preserves_existing_analysis_object() {
    let chapter = sample_chapter(Some(json!({
        "metadata": { "characters": ["林岚"] },
        "summaries": { "compressed": "重复摘要" },
        "key_events": ["发现线索"]
    })));

    let entry = sample_chapter_entry(&chapter).unwrap();

    assert_eq!(entry["analysis"]["metadata"]["characters"][0], "林岚");
    assert_eq!(entry["analysis"]["key_events"][0], "发现线索");
    assert!(entry["analysis"].get("summaries").is_none());
}

#[test]
fn sample_chapter_entry_rejects_non_object_analysis() {
    let chapter = sample_chapter(Some(json!([])));

    let err = sample_chapter_entry(&chapter).unwrap_err();

    assert!(err.contains("analysis_data 必须是对象"));
}

#[test]
fn chapter_summary_omits_missing_analysis_lists() {
    let chapter = sample_chapter(None);

    let entry = chapter_summary_for_reverse(&chapter);

    assert_eq!(entry["chapter_number"], 2);
    assert!(entry.get("summary").is_none());
    assert_eq!(entry["content_preview"], "第一段 第二段");
    assert!(entry.get("key_characters").is_none());
    assert!(entry.get("key_events").is_none());
}

#[test]
fn chapter_summary_preserves_real_summary() {
    let mut chapter = sample_chapter(None);
    chapter.summary = Some("林岚发现旧城线索。".to_string());

    let entry = chapter_summary_for_reverse(&chapter);

    assert_eq!(entry["summary"], "林岚发现旧城线索。");
    assert!(entry.get("content_preview").is_none());
}

#[test]
fn chapter_summary_omits_placeholder_title() {
    let mut chapter = sample_chapter(None);
    chapter.title = Some(" ".to_string());

    let entry = chapter_summary_for_reverse(&chapter);

    assert!(entry.get("title").is_none());
}

#[test]
fn chapter_summary_preserves_existing_analysis_lists() {
    let chapter = sample_chapter(Some(json!({
        "metadata": { "characters": ["林岚", "周澈"] },
        "key_events": [
            { "description": "林岚发现线索" },
            "周澈隐瞒真相"
        ]
    })));

    let entry = chapter_summary_for_reverse(&chapter);

    assert_eq!(entry["key_characters"][0], "林岚");
    assert_eq!(entry["key_characters"][1], "周澈");
    assert_eq!(entry["key_events"][0], "林岚发现线索");
    assert_eq!(entry["key_events"][1], "周澈隐瞒真相");
}

#[test]
fn collect_extracted_characters_merges_chapter_numbers_for_repeated_character() {
    let chapters = vec![
        sample_chapter_with_number(
            1,
            Some(json!({
                "metadata": { "characters": ["林岚"] }
            })),
        ),
        sample_chapter_with_number(
            2,
            Some(json!({
                "metadata": { "characters": ["林岚"] }
            })),
        ),
    ];

    let characters = collect_extracted_characters(&chapters, 40);

    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0]["name"], "林岚");
    assert_eq!(characters[0]["chapters"], json!([1, 2]));
}

#[test]
fn collect_extracted_characters_keeps_merging_existing_names_after_limit() {
    let chapters = vec![
        sample_chapter_with_number(
            1,
            Some(json!({
                "metadata": { "characters": ["林岚", "周澈"] }
            })),
        ),
        sample_chapter_with_number(
            2,
            Some(json!({
                "metadata": { "characters": ["周澈", "林岚"] }
            })),
        ),
    ];

    let characters = collect_extracted_characters(&chapters, 1);

    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0]["name"], "林岚");
    assert_eq!(characters[0]["chapters"], json!([1, 2]));
}

#[test]
fn collect_extracted_characters_trims_object_character_names() {
    let chapter = sample_chapter(Some(json!({
        "metadata": {
            "characters": [
                {"name": " 林岚 ", "role": "主角"}
            ]
        }
    })));

    let characters = collect_extracted_characters(&[chapter], 40);

    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0]["name"], "林岚");
    assert_eq!(characters[0]["role"], "主角");
}

#[test]
fn dedup_character_records_trims_kept_character_name() {
    let records = dedup_character_records(vec![
        json!({"name": " 林岚 ", "chapters": [1]}),
        json!({"name": "林岚", "chapters": [2]}),
    ]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["name"], "林岚");
    assert_eq!(records[0]["chapters"], json!([1]));
}
