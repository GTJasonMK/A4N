use serde_json::json;

use super::{chapter_summary_for_reverse, sample_chapter_entry};
use crate::import_reverse::context::test_support::sample_chapter;

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
