use super::super::*;
use super::fixtures::{canonical_character_sync_value, parse_sync_value};
use serde_json::{Value, json};

#[test]
fn parse_character_sync_output_accepts_canonical_object() {
    let value = parse_sync_value(canonical_character_sync_value())
        .expect("canonical sync output should parse");
    assert_eq!(value["attribute_changes"][0]["new_value"], "冷静");
}

#[test]
fn parse_character_sync_output_rejects_direct_attribute_delete() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"][0]["key"] = json!("护身玉佩");
    value["attribute_changes"][0]["operation"] = json!("delete");
    value["attribute_changes"][0]["new_value"] = Value::Null;
    value["attribute_changes"][0]["change_description"] = json!("玉佩损毁");
    value["attribute_changes"][0]["event_cause"] = json!("战斗中损毁");
    value["attribute_changes"][0]["evidence_anchor_id"] = json!(2);

    let err = parse_sync_value(value)
        .expect_err("direct attribute delete must go through deletion candidates");
    assert!(err.contains("deletion_candidates"));
}

#[test]
fn parse_character_sync_output_rejects_llm_old_value() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"][0]["old_value"] = json!("谨慎");

    let err = parse_sync_value(value).expect_err("old_value must be system-computed");
    assert!(err.contains("old_value 由系统计算"));
}

#[test]
fn parse_character_sync_output_rejects_non_numeric_attribute_evidence_anchor_id() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"][0]["evidence_anchor_id"] = json!("ch1:s1:f1");

    let err = parse_sync_value(value).expect_err("attribute evidence anchor id must be numeric");
    assert!(err.contains("evidence_anchor_id"));
    assert!(err.contains("JSON 整数数字"));
}

#[test]
fn parse_character_sync_output_rejects_non_positive_evidence_anchor_id() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"][0]["evidence_anchor_id"] = json!(0);

    let err = parse_sync_value(value).expect_err("non-positive evidence anchor id must fail");

    assert!(err.contains("attribute_changes[0].evidence_anchor_id"));
    assert!(err.contains("必须大于 0"));
}

#[test]
fn parse_character_sync_output_rejects_legacy_evidence_fields() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"][0]["evidence"] = json!("他沉默片刻后才开口");
    value["behaviors"][0]["original_text"] = json!("他沉默片刻后才开口");
    value["deletion_candidates"] = json!([{
        "chapter_number": 1,
        "category": "explicit",
        "key": "appearance",
        "reason": "旧外观需要删除",
        "evidence_anchor_id": 1,
        "evidence": "他沉默片刻后才开口"
    }]);

    let err = parse_sync_value(value).expect_err("legacy evidence fields must fail");

    assert!(err.contains("evidence 已废弃"));
}

#[test]
fn parse_character_sync_output_rejects_null_attribute_new_value() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"][0]["new_value"] = Value::Null;

    let err = parse_sync_value(value).expect_err("add/modify new_value must be non-null");

    assert!(err.contains("new_value 不能为空"));
}

#[test]
fn parse_character_sync_output_rejects_non_string_behavior_tags() {
    let mut value = canonical_character_sync_value();
    value["behaviors"][0]["behavior_tags"] = json!(["谨慎", 7]);

    let err = parse_sync_value(value).expect_err("behavior tags must be strings");

    assert!(err.contains("behavior_tags[1] 必须是字符串"));
}

#[test]
fn parse_character_sync_output_rejects_non_string_related_characters() {
    let mut value = canonical_character_sync_value();
    value["behaviors"][0]["related_characters"] = json!(["少女", {"name": "沈砚"}]);

    let err = parse_sync_value(value).expect_err("related characters must be strings");

    assert!(err.contains("related_characters[1] 必须是字符串"));
}

#[test]
fn parse_character_sync_output_rejects_string_chapter_number_with_path() {
    let mut value = canonical_character_sync_value();
    value["behaviors"][0]["chapter_number"] = json!("第1章");

    let err = parse_sync_value(value).expect_err("string chapter_number must fail");

    assert!(err.contains("behaviors[0].chapter_number"));
    assert!(err.contains("JSON 整数数字"));
    assert!(err.contains("第…章"));
}

#[test]
fn parse_character_sync_output_rejects_invalid_deletion_evidence_anchor_id() {
    let mut value = canonical_character_sync_value();
    value["deletion_candidates"] = json!([{
        "chapter_number": 1,
        "category": "explicit",
        "key": "appearance",
        "reason": "本章未再出现",
        "evidence_anchor_id": 0
    }]);

    let err = parse_sync_value(value).expect_err("deletion evidence anchor id must be valid");
    assert!(err.contains("evidence_anchor_id"));
}

#[test]
fn parse_character_sync_output_rejects_duplicate_deletion_candidates() {
    let mut value = canonical_character_sync_value();
    value["deletion_candidates"] = json!([
        {
            "chapter_number": 1,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩损毁",
            "evidence_anchor_id": 2
        },
        {
            "chapter_number": 1,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩彻底损毁",
            "evidence_anchor_id": 3
        }
    ]);

    let err = parse_sync_value(value)
        .expect_err("duplicate deletion candidates should be rejected at parse boundary");

    assert!(err.contains("删除候选重复"));
}

#[test]
fn parse_character_sync_output_rejects_deprecated_updated_attributes() {
    let mut value = canonical_character_sync_value();
    value["updated_attributes"] = json!({
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {}
    });

    let err = parse_sync_value(value).expect_err("updated_attributes must be system-computed");
    assert!(err.contains("updated_attributes 已废弃"));
}

#[test]
fn parse_character_sync_output_rejects_unknown_top_level_field() {
    let mut value = canonical_character_sync_value();
    value["final_attributes"] = json!({});

    let err = parse_sync_value(value).expect_err("unknown top-level field must fail");

    assert!(err.contains("final_attributes"));
    assert!(err.contains("不在当前角色同步合同中"));
}

#[test]
fn parse_character_sync_output_rejects_markdown_fence() {
    let err = parse_character_sync_output(
            "```json\n{\"attribute_changes\":[],\"behaviors\":[],\"deletion_candidates\":[],\"chapter_states\":[{\"chapter_number\":1,\"state_summary\":\"冷静\",\"current_location\":null,\"current_goal\":null,\"emotional_state\":null}]}\n```",
        )
        .expect_err("markdown fence must fail");
    assert!(err.contains("JSON 对象"));
}

#[test]
fn parse_character_sync_output_rejects_inline_classification_results() {
    let mut value = canonical_character_sync_value();
    value["attribute_changes"] = json!([]);
    value["behaviors"][0]["classification_results"] = json!({"谨慎":"conform"});

    let err = parse_sync_value(value).expect_err("inline classification_results must fail");
    assert!(err.contains("不应包含 classification_results"));
}

#[test]
fn parse_character_sync_output_rejects_legacy_top_level_state() {
    let mut value = canonical_character_sync_value();
    value.as_object_mut().unwrap().remove("chapter_states");
    value["state_summary"] = json!("冷静寻找线索");
    value["current_location"] = Value::Null;
    value["current_goal"] = json!("寻找线索");
    value["emotional_state"] = json!("紧张");

    let err = parse_sync_value(value).expect_err("legacy top-level state must fail");
    assert!(err.contains("顶层字段 state_summary 已废弃"));
}

#[test]
fn chapter_state_drafts_cover_every_selected_chapter() {
    let mut value = canonical_character_sync_value();
    value["chapter_states"] = json!([
        {
            "chapter_number": 2,
            "state_summary": "抵达镇外",
            "current_location": "镇外",
            "current_goal": "入镇调查",
            "emotional_state": "警惕"
        },
        {
            "chapter_number": 1,
            "state_summary": "离开山门",
            "current_location": "山门",
            "current_goal": "赶往落霞镇",
            "emotional_state": "坚定"
        }
    ]);

    let drafts =
        chapter_state_drafts(&value, &[1, 2]).expect("states should be ordered by input chapters");
    assert_eq!(drafts[0].chapter_number, 1);
    assert_eq!(drafts[0].state_summary, "离开山门");
    assert_eq!(drafts[1].chapter_number, 2);
    assert_eq!(drafts[1].current_location.as_deref(), Some("镇外"));
}

#[test]
fn chapter_state_drafts_rejects_missing_chapter() {
    let value = canonical_character_sync_value();

    let err = chapter_state_drafts(&value, &[1, 2])
        .expect_err("missing selected chapter state must fail");
    assert!(err.contains("缺少章节状态"));
    assert!(err.contains("第 2 章"));
}

#[test]
fn chapter_state_drafts_rejects_duplicate_chapter() {
    let mut value = canonical_character_sync_value();
    value["chapter_states"] = json!([
        {
            "chapter_number": 1,
            "state_summary": "第一次状态",
            "current_location": null,
            "current_goal": null,
            "emotional_state": null
        },
        {
            "chapter_number": 1,
            "state_summary": "重复状态",
            "current_location": null,
            "current_goal": null,
            "emotional_state": null
        }
    ]);

    let err = chapter_state_drafts(&value, &[1]).expect_err("duplicate chapter state must fail");
    assert!(err.contains("重复章节"));
}

#[test]
fn chapter_state_drafts_rejects_out_of_range_chapter() {
    let mut value = canonical_character_sync_value();
    value["chapter_states"][0]["chapter_number"] = json!(3);

    let err = chapter_state_drafts(&value, &[1, 2]).expect_err("out-of-range chapter must fail");
    assert!(err.contains("超出本次同步章节"));
    assert!(err.contains("第 3 章"));
}

#[test]
fn validate_unique_deletion_candidates_rejects_duplicate_in_same_chapter() {
    let candidates = vec![
        json!({
            "chapter_number": 3,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩损毁",
            "evidence_anchor_id": 31
        }),
        json!({
            "chapter_number": 3,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩彻底损毁",
            "evidence_anchor_id": 32
        }),
    ];

    let err = validate_unique_deletion_candidates(&candidates)
        .expect_err("duplicate deletion candidate in one chapter must fail");

    assert!(err.contains("删除候选重复"));
    assert!(err.contains("第 3 章 explicit.护身玉佩"));
}

#[test]
fn validate_unique_deletion_candidates_allows_consecutive_chapter_marks() {
    let candidates = vec![
        json!({
            "chapter_number": 3,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩裂开",
            "evidence_anchor_id": 31
        }),
        json!({
            "chapter_number": 4,
            "category": "explicit",
            "key": "护身玉佩",
            "reason": "玉佩仍不可用",
            "evidence_anchor_id": 41
        }),
    ];

    validate_unique_deletion_candidates(&candidates)
        .expect("same attribute across chapters should count as deletion-protection timeline");
}
