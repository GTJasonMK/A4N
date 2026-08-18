use a4n_core::{
    models::{character::protagonist_profile, project::blueprint_character},
    rag::RagDataType,
};
use a4n_orchestrator::rag::{blueprint_character_rag_records, protagonist_rag_records};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::json;

fn ts() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid date")
        .and_hms_opt(0, 0, 0)
        .expect("valid time")
}

fn sample_profile() -> protagonist_profile::Model {
    protagonist_profile::Model {
        id: 40,
        project_id: 1,
        name: "林舟".to_string(),
        gender: Some("女".to_string()),
        age: Some("29".to_string()),
        base_attributes: json!({"desire": "自由"}),
        current_state: json!({"location": "边境城"}),
        character_arc: Some(json!({"start": "逃避", "end": "承担"})),
        voice_fingerprint: Some(json!({"voice_style": "简短直接"})),
        appearance: Some("短发，旧外套".to_string()),
        background: Some("前领航员".to_string()),
        personality: Some("谨慎".to_string()),
        last_synced_chapter: 0,
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_blueprint_character() -> blueprint_character::Model {
    blueprint_character::Model {
        id: 70,
        project_id: 1,
        name: "沈砚".to_string(),
        character_type: "supporting".to_string(),
        gender: Some("男".to_string()),
        age: Some("34".to_string()),
        appearance: Some("灰色风衣".to_string()),
        personality: Some("克制".to_string()),
        background: Some("旧航线调查员".to_string()),
        motivation: Some("查清星门事故".to_string()),
        arc_summary: Some("从旁观到介入".to_string()),
        relationship_to_protagonist: Some("临时盟友".to_string()),
        arc_stages: Some(json!([{"turning_point": "共同脱险"}])),
        created_at: ts(),
        updated_at: ts(),
    }
}

#[test]
fn blueprint_character_records_reject_missing_identity_contract() {
    let mut character = sample_blueprint_character();
    character.name = " ".to_string();
    assert!(blueprint_character_rag_records(9, &character).is_err());

    character = sample_blueprint_character();
    character.character_type = " ".to_string();
    assert!(blueprint_character_rag_records(9, &character).is_err());
}

#[test]
fn blueprint_character_records_preserve_dimensions_ids_and_metadata() {
    let character = sample_blueprint_character();
    let records = blueprint_character_rag_records(9, &character).expect("character records");
    assert_eq!(records.len(), 7);

    let motivation = records
        .iter()
        .find(|record| record.chunk_id == "character:9:blueprint:70:motivation")
        .expect("motivation record");
    assert_eq!(motivation.chunk_id, "character:9:blueprint:70:motivation");
    assert!(motivation.text.contains("[角色设定-动机]"));
    assert!(motivation.text.contains("动机: 查清星门事故"));
    assert_eq!(
        motivation.metadata["data_type"],
        RagDataType::Character.as_str()
    );
    assert_eq!(motivation.metadata["title"], "沈砚 角色设定: 动机");
}

#[test]
fn blueprint_character_records_omit_blank_optional_dimensions() {
    let mut character = sample_blueprint_character();
    character.gender = None;
    character.age = Some(" ".to_string());
    character.appearance = Some(" ".to_string());
    character.personality = None;
    character.background = None;
    character.motivation = None;
    character.arc_summary = None;
    character.relationship_to_protagonist = None;
    character.arc_stages = None;

    let records = blueprint_character_rag_records(9, &character).expect("identity record");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].chunk_id, "character:9:blueprint:70:identity");
    assert_eq!(
        records[0].text,
        "[角色设定-身份]\n角色: 沈砚\n类型: supporting"
    );
}

#[test]
fn protagonist_records_reject_blank_name() {
    let mut profile = sample_profile();
    profile.name = " ".to_string();
    assert!(protagonist_rag_records(9, &profile).is_err());
}

#[test]
fn protagonist_records_preserve_dimensions_ids_and_metadata() {
    let profile = sample_profile();
    let records = protagonist_rag_records(9, &profile).expect("protagonist records");
    assert_eq!(records.len(), 8);

    let current_state = records
        .iter()
        .find(|record| record.chunk_id == "protagonist:9:40:current_state")
        .expect("current state record");
    assert_eq!(current_state.chunk_id, "protagonist:9:40:current_state");
    assert!(current_state.text.contains("[主角档案-当前状态]"));
    assert!(current_state.text.contains("位置: 边境城"));
    assert_eq!(
        current_state.metadata["data_type"],
        RagDataType::Protagonist.as_str()
    );
    assert_eq!(current_state.metadata["title"], "林舟 主角档案: 当前状态");
}

#[test]
fn protagonist_records_omit_blank_optional_dimensions() {
    let mut profile = sample_profile();
    profile.gender = None;
    profile.age = Some(" ".to_string());
    profile.appearance = Some(" ".to_string());
    profile.background = None;
    profile.personality = None;
    profile.base_attributes = json!({});
    profile.current_state = json!({});
    profile.character_arc = None;
    profile.voice_fingerprint = None;

    let records = protagonist_rag_records(9, &profile).expect("identity record");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].chunk_id, "protagonist:9:40:identity");
    assert_eq!(records[0].text, "[主角档案-身份]\n角色: 林舟");
}
