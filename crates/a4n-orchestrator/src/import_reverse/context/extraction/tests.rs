use serde_json::json;

use super::{collect_extracted_characters, dedup_character_records};
use crate::import_reverse::context::test_support::{sample_chapter, sample_chapter_with_number};

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
