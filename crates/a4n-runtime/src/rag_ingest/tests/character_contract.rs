use super::super::record_builders::{
    blueprint_character_rag_records, blueprint_character_rag_sections,
    blueprint_character_rag_text, character_state_rag_text,
    expected_blueprint_character_record_count, expected_protagonist_record_count,
    foreshadowing_rag_record, foreshadowing_rag_text, protagonist_rag_records,
    protagonist_rag_sections, protagonist_rag_text,
};
use super::{sample_blueprint_character, sample_foreshadowing, sample_profile, sample_state_index};
use a4n_core::rag::RagDataType;
use serde_json::json;

#[test]
fn protagonist_rag_text_requires_name() {
    let mut profile = sample_profile();
    profile.name = " ".to_string();

    let err = protagonist_rag_text(&profile).expect_err("blank name should reject");

    assert!(err.contains("缺少姓名"));
}

#[test]
fn protagonist_rag_text_omits_missing_json_sections() {
    let mut profile = sample_profile();
    profile.character_arc = None;
    profile.voice_fingerprint = None;

    let text = protagonist_rag_text(&profile).expect("valid profile text");

    assert!(text.contains("角色: 林舟"));
    assert!(text.contains("基础属性:"));
    assert!(text.contains("当前状态:"));
    assert!(!text.contains("角色弧光:"));
    assert!(!text.contains("声音指纹:"));
}

#[test]
fn protagonist_rag_sections_split_profile_by_dimension() {
    let sections = protagonist_rag_sections(&sample_profile()).expect("valid profile sections");

    let dimensions = sections
        .iter()
        .map(|section| section.dimension)
        .collect::<Vec<_>>();
    assert_eq!(
        dimensions,
        vec![
            "identity",
            "appearance",
            "background",
            "personality",
            "base_attributes",
            "current_state",
            "character_arc",
            "voice_fingerprint"
        ]
    );
    let personality = sections
        .iter()
        .find(|section| section.dimension == "personality")
        .expect("personality section");
    assert!(personality.text.contains("[主角档案-人格]"));
    assert!(personality.text.contains("角色: 林舟"));
    assert!(personality.text.contains("人格: 谨慎"));
}

#[test]
fn protagonist_rag_sections_omit_blank_optional_dimensions() {
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

    let sections = protagonist_rag_sections(&profile).expect("identity section still valid");

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].dimension, "identity");
    assert_eq!(sections[0].text, "[主角档案-身份]\n角色: 林舟");
}

#[test]
fn protagonist_rag_records_use_dimension_ids_and_metadata() {
    let records = protagonist_rag_records(9, &sample_profile()).expect("valid records");
    let state_record = records
        .iter()
        .find(|record| record.metadata["dimension"] == json!("current_state"))
        .expect("current state record");

    assert_eq!(state_record.chunk_id, "protagonist:9:40:current_state");
    assert!(state_record.text.contains("[主角档案-当前状态]"));
    assert!(state_record.text.contains("角色: 林舟"));
    assert!(state_record.text.contains("位置: 边境城"));
    assert_eq!(
        state_record.metadata["data_type"],
        json!(RagDataType::Protagonist.as_str())
    );
    assert_eq!(state_record.metadata["profile_id"], json!(40));
    assert_eq!(state_record.metadata["character_name"], json!("林舟"));
    assert_eq!(
        state_record.metadata["title"],
        json!("林舟 主角档案: 当前状态")
    );
}

#[test]
fn expected_protagonist_count_uses_dimension_records() {
    let mut profile = sample_profile();
    assert_eq!(expected_protagonist_record_count(9, &profile), 8);

    profile.appearance = Some(" ".to_string());
    profile.background = None;
    profile.voice_fingerprint = None;
    assert_eq!(expected_protagonist_record_count(9, &profile), 5);

    profile.name = " ".to_string();
    assert_eq!(expected_protagonist_record_count(9, &profile), 0);
}

#[test]
fn blueprint_character_rag_text_requires_name() {
    let mut character = sample_blueprint_character();
    character.name = " ".to_string();

    let err = blueprint_character_rag_text(&character).expect_err("blank name should reject");

    assert!(err.contains("缺少姓名"));
}

#[test]
fn blueprint_character_rag_text_requires_type() {
    let mut character = sample_blueprint_character();
    character.character_type = " ".to_string();

    let err = blueprint_character_rag_text(&character).expect_err("blank type should reject");

    assert!(err.contains("缺少类型"));
}

#[test]
fn blueprint_character_rag_text_includes_real_fields() {
    let text =
        blueprint_character_rag_text(&sample_blueprint_character()).expect("valid character text");

    assert!(text.contains("角色: 沈砚"));
    assert!(text.contains("类型: supporting"));
    assert!(text.contains("动机: 查清星门事故"));
    assert!(text.contains("内在弧线:"));
}

#[test]
fn blueprint_character_rag_sections_require_type() {
    let mut character = sample_blueprint_character();
    character.character_type = " ".to_string();

    let err = blueprint_character_rag_sections(&character).expect_err("blank type should reject");

    assert!(err.contains("缺少类型"));
}

#[test]
fn blueprint_character_rag_sections_split_character_by_dimension() {
    let sections =
        blueprint_character_rag_sections(&sample_blueprint_character()).expect("valid sections");

    let dimensions = sections
        .iter()
        .map(|section| section.dimension)
        .collect::<Vec<_>>();
    assert_eq!(
        dimensions,
        vec![
            "identity",
            "appearance",
            "personality",
            "background",
            "motivation",
            "arc",
            "relationship_to_protagonist"
        ]
    );
    let arc = sections
        .iter()
        .find(|section| section.dimension == "arc")
        .expect("arc section");
    assert!(arc.text.contains("[角色设定-角色弧光]"));
    assert!(arc.text.contains("角色: 沈砚"));
    assert!(arc.text.contains("角色弧光: 从旁观到介入"));
    assert!(arc.text.contains("内在弧线:"));
}

#[test]
fn blueprint_character_rag_sections_omit_blank_optional_dimensions() {
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

    let sections = blueprint_character_rag_sections(&character)
        .expect("identity section still valid because type is required");

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].dimension, "identity");
    assert_eq!(
        sections[0].text,
        "[角色设定-身份]\n角色: 沈砚\n类型: supporting"
    );
}

#[test]
fn blueprint_character_rag_records_use_dimension_ids_and_metadata() {
    let records =
        blueprint_character_rag_records(9, &sample_blueprint_character()).expect("valid records");
    let motivation = records
        .iter()
        .find(|record| record.metadata["dimension"] == json!("motivation"))
        .expect("motivation record");

    assert_eq!(motivation.chunk_id, "character:9:blueprint:70:motivation");
    assert!(motivation.text.contains("[角色设定-动机]"));
    assert!(motivation.text.contains("角色: 沈砚"));
    assert!(motivation.text.contains("动机: 查清星门事故"));
    assert_eq!(
        motivation.metadata["data_type"],
        json!(RagDataType::Character.as_str())
    );
    assert_eq!(motivation.metadata["source"], json!("blueprint_character"));
    assert_eq!(motivation.metadata["character_id"], json!(70));
    assert_eq!(motivation.metadata["character_name"], json!("沈砚"));
    assert_eq!(motivation.metadata["title"], json!("沈砚 角色设定: 动机"));
}

#[test]
fn expected_blueprint_character_count_uses_dimension_records() {
    let mut character = sample_blueprint_character();
    assert_eq!(expected_blueprint_character_record_count(9, &character), 7);

    character.appearance = Some(" ".to_string());
    character.personality = None;
    character.background = None;
    character.motivation = None;
    character.arc_summary = None;
    character.arc_stages = None;
    character.relationship_to_protagonist = None;
    assert_eq!(expected_blueprint_character_record_count(9, &character), 1);

    character.character_type = " ".to_string();
    assert_eq!(expected_blueprint_character_record_count(9, &character), 0);
}

#[test]
fn character_state_rag_text_requires_name() {
    let state_index = sample_state_index();

    let err = character_state_rag_text(&state_index, " ").expect_err("blank name should reject");

    assert!(err.contains("缺少角色名"));
}

#[test]
fn character_state_rag_text_requires_state_summary() {
    let mut state_index = sample_state_index();
    state_index.state_summary = " ".to_string();

    let err =
        character_state_rag_text(&state_index, "林舟").expect_err("blank summary should reject");

    assert!(err.contains("缺少状态摘要"));
}

#[test]
fn foreshadowing_rag_text_requires_description() {
    let mut item = sample_foreshadowing();
    item.description = " ".to_string();

    let err = foreshadowing_rag_text(&item).expect_err("blank description should reject");

    assert!(err.contains("缺少描述"));
}

#[test]
fn foreshadowing_rag_text_includes_status_chapters_importance_and_related_entities() {
    let text = foreshadowing_rag_text(&sample_foreshadowing()).expect("valid foreshadowing text");

    assert!(text.contains("伏笔: 旧导航图上的缺口指向被删除的星门坐标。"));
    assert!(text.contains("埋设章节: 第3章"));
    assert!(text.contains("揭示章节: 第12章"));
    assert!(text.contains("状态: pending"));
    assert!(text.contains("重要度: 5"));
    assert!(text.contains("关联角色/实体:"));
    assert!(text.contains("林舟"));
    assert!(text.contains("沈砚"));
}

#[test]
fn foreshadowing_rag_record_uses_independent_prefix_and_searchable_metadata() {
    let record =
        foreshadowing_rag_record(9, &sample_foreshadowing()).expect("valid foreshadowing record");

    assert_eq!(record.chunk_id, "foreshadowing:9:80");
    assert_eq!(
        record.metadata["data_type"],
        json!(RagDataType::Foreshadowing.as_str())
    );
    assert_eq!(record.metadata["foreshadowing_id"], json!(80));
    assert_eq!(record.metadata["chapter_number"], json!(3));
    assert_eq!(record.metadata["setup_chapter"], json!(3));
    assert_eq!(record.metadata["payoff_chapter"], json!(12));
    assert_eq!(record.metadata["status"], json!("pending"));
    assert_eq!(record.metadata["importance"], json!(5));
    assert!(
        record.metadata["title"]
            .as_str()
            .is_some_and(|title| title.starts_with("伏笔 第3章: "))
    );
}
