use a4n_db::rag::RagBlueprintRelationshipRow;
use a4n_domain::{project::BlueprintRecord, rag::RagDataType};
use a4n_orchestrator::rag::{
    blueprint_metadata_rag_record, blueprint_rag_record, blueprint_world_setting_rag_records,
    relationship_rag_record,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde_json::json;

fn ts() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid date")
        .and_hms_opt(0, 0, 0)
        .expect("valid time")
}

fn sample_blueprint() -> BlueprintRecord {
    BlueprintRecord {
        id: 30,
        project_id: 1,
        title: "星门余烬".to_string(),
        genre: "科幻".to_string(),
        style: Some("冷峻".to_string()),
        tone: Some("克制".to_string()),
        target_audience: Some("成年读者".to_string()),
        one_sentence_summary: Some("幸存者寻找失落星门。".to_string()),
        full_synopsis: Some("主角沿废弃航线追索星门真相。".to_string()),
        world_setting: Some(json!({"premise": "星门网络崩塌"})),
        current_version: "1.0.0".to_string(),
        needs_part_outlines: true,
        total_chapters: Some(24),
        chapters_per_part: Some(8),
        avatar_svg: None,
        avatar_animal: None,
        created_at: ts(),
        updated_at: ts(),
    }
}

fn sample_relationship() -> RagBlueprintRelationshipRow {
    RagBlueprintRelationshipRow {
        id: 60,
        relationship_type: "ally".to_string(),
        character_a: "林舟".to_string(),
        character_b: "沈砚".to_string(),
        description: Some("互相利用，逐步建立信任。".to_string()),
        strength: Some(6),
        evolution: Some(json!([{"event": "共同脱险", "impact": "信任上升"}])),
    }
}

#[test]
fn blueprint_record_requires_synopsis_and_preserves_canonical_contract() {
    let mut blueprint = sample_blueprint();
    blueprint.full_synopsis = Some(" ".to_string());
    assert!(blueprint_rag_record(9, &blueprint).is_err());

    let blueprint = sample_blueprint();
    let record = blueprint_rag_record(9, &blueprint).expect("blueprint record");
    assert_eq!(record.chunk_id, "blueprint:9:30");
    assert!(record.text.contains("标题: 星门余烬"));
    assert!(
        record
            .text
            .contains("完整梗概: 主角沿废弃航线追索星门真相。")
    );
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::Blueprint.as_str()
    );
}

#[test]
fn blueprint_metadata_record_requires_title_and_omits_blank_optional_labels() {
    let mut blueprint = sample_blueprint();
    blueprint.title = " ".to_string();
    assert!(blueprint_metadata_rag_record(9, &blueprint).is_err());

    blueprint = sample_blueprint();
    blueprint.style = Some(" ".to_string());
    blueprint.tone = None;
    blueprint.target_audience = None;
    blueprint.one_sentence_summary = None;
    let record = blueprint_metadata_rag_record(9, &blueprint).expect("metadata record");
    assert!(record.text.contains("小说标题: 星门余烬"));
    assert!(record.text.contains("体裁类型: 科幻"));
    assert!(!record.text.contains("写作风格:"));
    assert!(!record.text.contains("情感基调:"));
}

#[test]
fn world_setting_records_reject_missing_or_empty_content() {
    let mut blueprint = sample_blueprint();
    blueprint.world_setting = None;
    assert!(blueprint_world_setting_rag_records(9, &blueprint).is_err());

    blueprint.world_setting = Some(json!({"premise": " ", "core_rules": null}));
    assert!(blueprint_world_setting_rag_records(9, &blueprint).is_err());
}

#[test]
fn world_setting_records_preserve_sections_ids_and_metadata() {
    let mut blueprint = sample_blueprint();
    blueprint.world_setting = Some(json!({
        "premise": "星门网络崩塌",
        "core_rules": "跃迁必须消耗坐标钥匙",
        "ignored": null,
    }));
    let records = blueprint_world_setting_rag_records(9, &blueprint).expect("world records");
    assert_eq!(records.len(), 2);

    let premise = records
        .iter()
        .find(|record| record.chunk_id == "world_setting:9:30:premise")
        .expect("premise record");
    assert_eq!(premise.chunk_id, "world_setting:9:30:premise");
    assert!(premise.text.contains("[世界观-世界前提]"));
    assert_eq!(
        premise.metadata["data_type"],
        RagDataType::WorldSetting.as_str()
    );
}

#[test]
fn relationship_record_requires_description_and_preserves_contract() {
    let mut relationship = sample_relationship();
    relationship.description = None;
    assert!(relationship_rag_record(9, &relationship).is_err());

    let relationship = sample_relationship();
    let record = relationship_rag_record(9, &relationship).expect("relationship record");
    assert_eq!(record.chunk_id, "relationship:9:60");
    assert!(
        record
            .text
            .contains("林舟 -> 沈砚: 互相利用，逐步建立信任。")
    );
    assert!(record.text.contains("类型: ally"));
    assert!(record.text.contains("强度: 6"));
    assert_eq!(
        record.metadata["data_type"],
        RagDataType::Relationship.as_str()
    );
}
