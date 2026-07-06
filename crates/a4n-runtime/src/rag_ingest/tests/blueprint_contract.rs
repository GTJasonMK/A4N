use super::super::record_builders::{
    blueprint_metadata_rag_text, blueprint_rag_text, blueprint_world_setting_rag_records,
    blueprint_world_setting_sections, expected_blueprint_world_setting_record_count,
    relationship_rag_record, relationship_rag_text,
};
use super::{sample_blueprint, sample_relationship};
use a4n_core::rag::RagDataType;
use serde_json::json;

#[test]
fn blueprint_rag_text_requires_full_synopsis() {
    let mut blueprint = sample_blueprint();
    blueprint.full_synopsis = Some(" ".to_string());

    let err = blueprint_rag_text(&blueprint).expect_err("blank synopsis should reject");

    assert!(err.contains("缺少完整梗概"));
}

#[test]
fn blueprint_rag_text_includes_full_synopsis() {
    let text = blueprint_rag_text(&sample_blueprint()).expect("valid blueprint text");

    assert!(text.contains("标题: 星门余烬"));
    assert!(text.contains("完整梗概: 主角沿废弃航线追索星门真相。"));
}

#[test]
fn blueprint_world_setting_sections_require_world_setting() {
    let mut blueprint = sample_blueprint();
    blueprint.world_setting = None;

    let err = blueprint_world_setting_sections(&blueprint)
        .expect_err("missing world setting should reject");

    assert!(err.contains("缺少世界观"));
}

#[test]
fn blueprint_world_setting_sections_reject_empty_content() {
    let mut blueprint = sample_blueprint();
    blueprint.world_setting = Some(json!({"premise": " ", "core_rules": null}));

    let err = blueprint_world_setting_sections(&blueprint)
        .expect_err("empty world setting should reject");

    assert!(err.contains("没有可入库内容"));
}

#[test]
fn blueprint_world_setting_sections_include_real_sections() {
    let sections =
        blueprint_world_setting_sections(&sample_blueprint()).expect("valid world setting");

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].0, "premise");
    assert!(sections[0].1.contains("[世界观-世界前提]"));
    assert!(sections[0].1.contains("星门网络崩塌"));
}

#[test]
fn blueprint_world_setting_rag_records_use_section_ids_and_metadata() {
    let mut blueprint = sample_blueprint();
    blueprint.world_setting = Some(json!({
        "premise": "星门网络崩塌",
        "core_rules": "跃迁必须消耗坐标钥匙",
    }));

    let records =
        blueprint_world_setting_rag_records(9, &blueprint).expect("valid world setting records");
    let premise = records
        .iter()
        .find(|record| record.metadata["field"] == json!("premise"))
        .expect("premise record");

    assert_eq!(premise.chunk_id, "world_setting:9:30:premise");
    assert!(premise.text.contains("[世界观-世界前提]"));
    assert_eq!(
        premise.metadata["data_type"],
        json!(RagDataType::WorldSetting.as_str())
    );
    assert_eq!(premise.metadata["blueprint_id"], json!(30));
    assert!(premise.metadata["section_index"].as_u64().is_some());
}

#[test]
fn expected_world_setting_count_uses_real_section_count() {
    let mut blueprint = sample_blueprint();
    blueprint.world_setting = Some(json!({
        "premise": "星门网络崩塌",
        "core_rules": "跃迁必须消耗坐标钥匙",
        "empty": " ",
        "ignored": null,
    }));

    assert_eq!(expected_blueprint_world_setting_record_count(&blueprint), 2);

    blueprint.world_setting = Some(json!({"premise": " "}));
    assert_eq!(expected_blueprint_world_setting_record_count(&blueprint), 0);
}

#[test]
fn blueprint_metadata_rag_text_requires_title() {
    let mut blueprint = sample_blueprint();
    blueprint.title = " ".to_string();

    let err = blueprint_metadata_rag_text(&blueprint).expect_err("blank title should reject");

    assert!(err.contains("缺少标题"));
}

#[test]
fn blueprint_metadata_rag_text_omits_blank_optional_labels() {
    let mut blueprint = sample_blueprint();
    blueprint.style = Some(" ".to_string());
    blueprint.tone = None;
    blueprint.target_audience = None;
    blueprint.one_sentence_summary = None;

    let text = blueprint_metadata_rag_text(&blueprint).expect("valid metadata text");

    assert!(text.contains("小说标题: 星门余烬"));
    assert!(text.contains("体裁类型: 科幻"));
    assert!(!text.contains("写作风格:"));
    assert!(!text.contains("情感基调:"));
    assert!(!text.contains("目标读者:"));
    assert!(!text.contains("一句话简介:"));
}

#[test]
fn relationship_rag_text_requires_description() {
    let mut relationship = sample_relationship();
    relationship.description = None;

    let err = relationship_rag_text(&relationship).expect_err("missing description should reject");

    assert!(err.contains("缺少描述"));
}

#[test]
fn relationship_rag_text_includes_description_type_and_strength() {
    let text = relationship_rag_text(&sample_relationship()).expect("valid relationship text");

    assert!(text.contains("林舟 -> 沈砚: 互相利用，逐步建立信任。"));
    assert!(text.contains("类型: ally"));
    assert!(text.contains("强度: 6"));
}

#[test]
fn relationship_rag_record_uses_canonical_id_text_and_metadata() {
    let record = relationship_rag_record(9, &sample_relationship()).expect("valid relationship");

    assert_eq!(record.chunk_id, "relationship:9:60");
    assert!(record.text.contains("林舟 -> 沈砚"));
    assert_eq!(
        record.metadata["data_type"],
        json!(RagDataType::Relationship.as_str())
    );
    assert_eq!(record.metadata["relationship_id"], json!(60));
    assert_eq!(record.metadata["character_a"], json!("林舟"));
    assert_eq!(record.metadata["character_b"], json!("沈砚"));
}
