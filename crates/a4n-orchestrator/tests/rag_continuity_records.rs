use a4n_core::{
    models::character::{attribute_event, character_state_index, foreshadowing_index},
    rag::RagDataType,
};
use a4n_orchestrator::rag::{
    character_attribute_event_rag_record, character_state_rag_record, foreshadowing_rag_record,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::{Value, json};

fn ts() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid date")
        .and_hms_opt(0, 0, 0)
        .expect("valid time")
}

fn state_index() -> character_state_index::Model {
    character_state_index::Model {
        id: 50,
        profile_id: 40,
        chapter_number: 3,
        state_summary: "确认星门坐标可能仍有效。".to_string(),
        key_attributes: json!({"trust": "低"}),
        emotional_state: Some("警惕".to_string()),
        current_location: Some("边境城".to_string()),
        current_goal: Some("找到旧导航图".to_string()),
        created_at: ts(),
    }
}

fn attribute_event() -> attribute_event::Model {
    attribute_event::Model {
        id: 70,
        profile_id: 40,
        chapter_number: 3,
        timestamp: ts(),
        category: "implicit".to_string(),
        key: "trust".to_string(),
        operation: "modify".to_string(),
        change_description: "对沈砚的信任提高。".to_string(),
        old_value: json!("低"),
        new_value: json!("中"),
        cause: "共同脱险".to_string(),
        evidence: "林舟主动交出导航图。".to_string(),
    }
}

fn foreshadowing() -> foreshadowing_index::Model {
    foreshadowing_index::Model {
        id: 80,
        project_id: 1,
        description: "旧导航图上的缺口指向被删除的星门坐标。".to_string(),
        setup_chapter: 3,
        payoff_chapter: Some(12),
        status: "pending".to_string(),
        related_characters: Some(json!(["林舟", {"name": "沈砚", "role": "知情者"}])),
        importance: 5,
        created_at: ts(),
        updated_at: ts(),
    }
}

#[test]
fn character_state_contract_preserves_required_text_and_metadata() {
    let mut state = state_index();
    assert!(character_state_rag_record(9, &state, " ").is_err());
    state.state_summary = " ".to_string();
    assert!(character_state_rag_record(9, &state, "林舟").is_err());

    let state = state_index();
    let record = character_state_rag_record(9, &state, "林舟").expect("state record");
    assert_eq!(record.chunk_id, "character_state:9:50");
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::CharacterState.as_str()
    );
    assert!(record.text.contains("位置: 边境城"));
}

#[test]
fn attribute_event_contract_preserves_labels_values_and_metadata() {
    let event = attribute_event();
    let record =
        character_attribute_event_rag_record(9, &event, "林舟").expect("attribute event record");
    assert_eq!(record.chunk_id, "character_attribute_event:9:70");
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::CharacterAttributeEvent.as_str()
    );
    assert_eq!(record.metadata["attribute_key"], "trust");
    assert!(record.text.contains("类别: 隐性属性"));
    assert!(record.text.contains("操作: 修改"));
    assert!(record.text.contains("原值: 低"));
    assert!(record.text.contains("新值: 中"));
}

#[test]
fn foreshadowing_contract_preserves_text_id_and_searchable_metadata() {
    let mut item = foreshadowing();
    item.description = " ".to_string();
    assert!(foreshadowing_rag_record(9, &item).is_err());

    let item = foreshadowing();
    let record = foreshadowing_rag_record(9, &item).expect("foreshadowing record");
    assert_eq!(record.chunk_id, "foreshadowing:9:80");
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::Foreshadowing.as_str()
    );
    assert_eq!(record.metadata["payoff_chapter"], Value::from(12));
    assert!(record.text.contains("埋设章节: 第3章"));
    assert!(record.text.contains("揭示章节: 第12章"));
    assert!(record.text.contains("林舟"));
}
