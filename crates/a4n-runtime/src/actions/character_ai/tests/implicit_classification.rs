use super::super::*;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn implicit_classification_user_prompt_uses_json_payload() {
    let prompt = build_implicit_classification_prompt(
        "林夏",
        &json!({
            "chapter_number": 3,
            "behavior_type": "action",
            "behavior_description": "先观察巷口再行动",
            "original_text": "林夏停在巷口，确认无人埋伏。",
            "behavior_tags": ["谨慎"]
        }),
        &json!({"谨慎": "行动前先观察"}),
    )
    .expect("classification prompt should build");

    assert!(prompt.contains(r#""name": "implicit_attribute_classification""#));
    assert!(prompt.contains(r#""current_implicit_attributes""#));
    assert!(prompt.contains(r#""current_implicit_attribute_keys""#));
    assert!(prompt.contains("新隐性属性必须写入 suggested_new_attributes"));
    assert!(prompt.contains("classifications 必须是 {}"));
    assert!(prompt.contains(r#""behavior_tags": ["#));
    assert!(prompt.contains("不要输出 evidence"));
    assert!(prompt.contains("behavior.original_text 回填"));
    assert!(!prompt.contains("## 行为信息"));
}

#[test]
fn parse_implicit_classification_output_accepts_canonical_object() {
    let parsed = parse_implicit_classification_output(
        r#"{
              "classifications": {
                "谨慎": "non-conform",
                "重情义": "conform"
              },
              "reasoning": "行为保护他人但没有先观察环境。",
              "suggested_new_attributes": [
                {
                  "key": "见义勇为",
                  "value": "遇到弱者受威胁时主动保护"
                }
              ]
            }"#,
        &json!({
            "谨慎": "遇事先观察",
            "重情义": "愿意保护他人"
        }),
    )
    .expect("canonical classification output should parse");

    assert_eq!(parsed["classifications"]["谨慎"], json!("non-conform"));
    assert_eq!(parsed["classifications"]["重情义"], json!("conform"));
    assert_eq!(
        parsed["suggested_new_attributes"][0]["key"],
        json!("见义勇为")
    );
}

#[test]
fn parse_implicit_classification_output_rejects_legacy_evidence() {
    let err = parse_implicit_classification_output(
        r#"{
              "classifications": {},
              "reasoning": "行为体现新属性。",
              "suggested_new_attributes": [
                {
                  "key": "见义勇为",
                  "value": "遇到弱者受威胁时主动保护",
                  "evidence": "林逸挡在少女身前"
                }
              ]
            }"#,
        &json!({
            "谨慎": "遇事先观察"
        }),
    )
    .expect_err("legacy evidence should fail");

    assert!(err.contains("evidence 已废弃"));
}

#[test]
fn parse_implicit_classification_output_rejects_unknown_top_level_field() {
    let err = parse_implicit_classification_output(
        r#"{
              "classifications": {},
              "reasoning": "无相关属性。",
              "suggested_new_attributes": [],
              "notes": "多余说明"
            }"#,
        &json!({
            "谨慎": "遇事先观察"
        }),
    )
    .expect_err("unknown top-level field should fail");

    assert!(err.contains("未支持字段 `notes`"));
}

#[test]
fn parse_implicit_classification_output_rejects_unknown_suggestion_field() {
    let err = parse_implicit_classification_output(
        r#"{
              "classifications": {},
              "reasoning": "行为体现新属性。",
              "suggested_new_attributes": [
                {
                  "key": "果断",
                  "value": "遇事快速决定",
                  "confidence": 0.8
                }
              ]
            }"#,
        &json!({
            "谨慎": "遇事先观察"
        }),
    )
    .expect_err("unknown suggested attribute field should fail");

    assert!(err.contains("未支持字段 `confidence`"));
}

#[test]
fn parse_implicit_classification_output_rejects_unknown_attribute() {
    let err = parse_implicit_classification_output(
        r#"{
              "classifications": {
                "谨慎": "conform",
                "孤僻": "non-conform"
              },
              "reasoning": "包含不存在的属性。",
              "suggested_new_attributes": []
            }"#,
        &json!({
            "谨慎": "遇事先观察"
        }),
    )
    .expect_err("classification for unknown attribute must fail");

    assert!(err.contains("不属于当前隐性属性"));
}

#[test]
fn parse_implicit_classification_output_rejects_invalid_value() {
    let err = parse_implicit_classification_output(
        r#"{
              "classifications": {
                "谨慎": "unknown"
              },
              "reasoning": "分类值不合法。",
              "suggested_new_attributes": []
            }"#,
        &json!({
            "谨慎": "遇事先观察"
        }),
    )
    .expect_err("classification values must be strict");

    assert!(err.contains("conform/non-conform"));
}

#[test]
fn apply_implicit_classification_adds_new_attributes_once() {
    let mut behavior = json!({
        "chapter_number": 3,
        "behavior_type": "action",
        "behavior_description": "保护陌生少女",
        "original_text": "林逸挡在少女身前",
        "behavior_tags": ["保护", "冲动"],
        "classification_results": {},
        "related_characters": ["少女"],
        "impact_analysis": ""
    });
    let parsed = json!({
        "classifications": {
            "谨慎": "non-conform"
        },
        "reasoning": "保护他人但缺少观察。",
        "suggested_new_attributes": [
            {
                "key": "见义勇为",
                "value": "遇到弱者受威胁时主动保护"
            },
            {
                "key": "见义勇为",
                "value": "重复建议应被忽略"
            },
            {
                "key": "谨慎",
                "value": "已有属性应被忽略"
            }
        ]
    });
    let mut current_state = json!({
        "explicit_attributes": {},
        "implicit_attributes": {
            "谨慎": "遇事先观察"
        },
        "social_attributes": {}
    });
    let mut suggested_keys = BTreeSet::new();

    let suggestions = apply_implicit_classification_to_behavior(
        &mut behavior,
        &parsed,
        &mut current_state,
        &mut suggested_keys,
        7,
    )
    .expect("classification should apply");

    assert_eq!(
        behavior["classification_results"]["谨慎"],
        json!("non-conform")
    );
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0]["key"], json!("见义勇为"));
    assert_eq!(suggestions[0]["sequence"], json!(7));
    assert_eq!(suggestions[0]["evidence"], json!("林逸挡在少女身前"));
    assert_eq!(
        current_state["implicit_attributes"]["见义勇为"],
        json!("遇到弱者受威胁时主动保护")
    );
    assert_eq!(
        current_state["implicit_attributes"]["谨慎"],
        json!("遇事先观察")
    );
}
