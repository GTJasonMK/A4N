use serde_json::{Value, json};

use super::super::validate_saved_chapter_analysis;
use a4n_error::AppError;

fn valid_analysis() -> Value {
    json!({
        "metadata": {
            "characters": ["林岚"],
            "locations": ["旧城"],
            "items": [],
            "tags": ["转折"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "发现线索",
            "one_line": "旧城线索",
            "keywords": ["线索"]
        },
        "character_states": {
            "林岚": {
                "location": "旧城",
                "status": "追查",
                "changes": ["发现标记"],
                "emotional_state": "警觉"
            }
        },
        "foreshadowing": {
            "planted": [{
                "description": "敌人去向未明",
                "original_text": "原文",
                "category": "plot_twist",
                "priority": "high",
                "related_entities": ["林岚"]
            }],
            "resolved": [],
            "tensions": []
        },
        "key_events": [{
            "type": "discovery",
            "description": "发现标记",
            "importance": "high",
            "involved_characters": ["林岚"]
        }]
    })
}

#[test]
fn accepts_normalized_analysis_with_meta() {
    let mut value = valid_analysis();
    value["_meta"] = json!({"content_sha256": "hash"});
    validate_saved_chapter_analysis(&value).unwrap();
}

#[test]
fn rejects_missing_required_root_field() {
    let mut value = valid_analysis();
    value.as_object_mut().unwrap().remove("summaries");
    let error = validate_saved_chapter_analysis(&value).unwrap_err();
    assert!(matches!(error, AppError::InvalidArgument(message) if message.contains("summaries")));
}

#[test]
fn rejects_unknown_nested_field() {
    let mut value = valid_analysis();
    value["character_states"]["林岚"]["emotion"] = json!("旧字段");
    let error = validate_saved_chapter_analysis(&value).unwrap_err();
    assert!(
        matches!(error, AppError::InvalidArgument(message) if message.contains("未知字段 emotion"))
    );
}

#[test]
fn rejects_non_string_or_empty_foreshadowing_related_entity() {
    for invalid in [json!({"name": "沈砚"}), json!(" ")] {
        let mut value = valid_analysis();
        value["foreshadowing"]["planted"][0]["related_entities"] = json!([invalid]);
        let error = validate_saved_chapter_analysis(&value).unwrap_err();
        assert!(
            matches!(error, AppError::InvalidArgument(message) if message.contains("related_entities[0] 必须是非空字符串"))
        );
    }
}

#[test]
fn rejects_non_object_saved_analysis() {
    let error = validate_saved_chapter_analysis(&json!([])).unwrap_err();
    assert!(matches!(error, AppError::InvalidArgument(message) if message.contains("JSON 对象")));
}
