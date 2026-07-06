use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary, ParsedImplicitDecision,
};
use crate::actions::json_contract::reject_unknown_fields;

pub(crate) fn parse_implicit_update_decision(
    content: &str,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<ParsedImplicitDecision, String> {
    let value: serde_json::Value = serde_json::from_str(content.trim())
        .map_err(|e| format!("隐性属性复审结果必须是 JSON 对象: {e}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "隐性属性复审结果必须是 JSON 对象".to_string())?;
    let decision = required_json_string(object, "decision")?;
    if !matches!(decision.as_str(), "keep" | "modify" | "delete") {
        return Err("隐性属性复审 decision 必须是 keep/modify/delete".to_string());
    }
    let reasoning = required_json_string(object, "reasoning")?;
    if object.contains_key("evidence_summary") {
        return Err(
            "隐性属性复审 evidence_summary 已废弃；必须输出 supporting_behavior_ids".to_string(),
        );
    }
    reject_unknown_fields(
        &value,
        "implicit_update",
        &[
            "decision",
            "reasoning",
            "new_value",
            "supporting_behavior_ids",
        ],
    )?;
    let supporting_behavior_ids = required_supporting_behavior_ids(object, stats)?;
    let evidence_summary = materialize_evidence_summary(&supporting_behavior_ids, &stats.evidence)?;
    let new_value = object
        .get("new_value")
        .ok_or_else(|| "隐性属性复审结果缺少字段: new_value".to_string())?;
    if matches!(decision.as_str(), "keep" | "delete") && !new_value.is_null() {
        return Err("keep/delete 决策的 new_value 必须为 null".to_string());
    }
    if decision == "modify" && new_value.is_null() {
        return Err("modify 决策的 new_value 不能为空".to_string());
    }

    Ok(ParsedImplicitDecision {
        decision,
        reasoning,
        suggested_new_value: if new_value.is_null() {
            None
        } else {
            Some(new_value.clone())
        },
        evidence_summary,
    })
}

fn required_supporting_behavior_ids(
    object: &serde_json::Map<String, serde_json::Value>,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<Vec<i64>, String> {
    let values = object
        .get("supporting_behavior_ids")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "隐性属性复审字段 supporting_behavior_ids 必须是数组".to_string())?;
    if values.is_empty() && !stats.evidence.is_empty() {
        return Err("隐性属性复审 supporting_behavior_ids 不能为空".to_string());
    }
    let candidates = stats
        .evidence
        .iter()
        .map(|item| item.behavior_id)
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value
            .as_i64()
            .ok_or_else(|| format!("隐性属性复审 supporting_behavior_ids[{index}] 必须是整数"))?;
        if !candidates.contains(&id) {
            return Err(format!(
                "隐性属性复审 supporting_behavior_ids[{index}] 不在输入 evidence 候选中: {id}"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "隐性属性复审 supporting_behavior_ids[{index}] 重复: {id}"
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn materialize_evidence_summary(
    ids: &[i64],
    evidence: &[ImplicitEvidenceSummary],
) -> Result<String, String> {
    if ids.is_empty() {
        return Ok("无可引用行为证据".to_string());
    }
    let by_id = evidence
        .iter()
        .map(|item| (item.behavior_id, item))
        .collect::<BTreeMap<_, _>>();
    ids.iter()
        .map(|id| {
            let item = by_id
                .get(id)
                .ok_or_else(|| format!("隐性属性复审支持行为不存在: {id}"))?;
            Ok(format_evidence_item(item))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|items| items.join("；"))
}

fn format_evidence_item(item: &ImplicitEvidenceSummary) -> String {
    let classification = item.classification.as_str().unwrap_or("unknown");
    format!(
        "行为{} / 第{}章：{}（{}）",
        item.behavior_id,
        item.chapter_number,
        item.behavior_description.trim(),
        classification
    )
}

fn required_json_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<String, String> {
    let text = object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("隐性属性复审字段 {key} 必须是字符串"))?
        .trim();
    if text.is_empty() {
        return Err(format!("隐性属性复审字段 {key} 不能为空"));
    }
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn stats() -> ImplicitAttributeReviewSummary {
        ImplicitAttributeReviewSummary {
            attribute_key: "谨慎".to_string(),
            current_value: json!("遇事先观察"),
            total: 2,
            conform_count: 0,
            non_conform_count: 2,
            conform_rate: 0.0,
            threshold_reached: true,
            evidence: vec![
                ImplicitEvidenceSummary {
                    behavior_id: 11,
                    chapter_number: 3,
                    behavior_type: "decision".to_string(),
                    behavior_description: "主动推进调查".to_string(),
                    original_text: "她没有继续等待。".to_string(),
                    behavior_tags: json!(["主动"]),
                    classification: json!("non-conform"),
                    created_at: None,
                },
                ImplicitEvidenceSummary {
                    behavior_id: 12,
                    chapter_number: 4,
                    behavior_type: "action".to_string(),
                    behavior_description: "直接进入危险区域".to_string(),
                    original_text: "她推门而入。".to_string(),
                    behavior_tags: json!(["果断"]),
                    classification: json!("non-conform"),
                    created_at: None,
                },
            ],
        }
    }

    #[test]
    fn parse_implicit_update_decision_accepts_modify() {
        let parsed = parse_implicit_update_decision(
            r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11,12]}"#,
            &stats(),
        )
        .expect("modify decision should parse");

        assert_eq!(parsed.decision, "modify");
        assert_eq!(parsed.suggested_new_value, Some(json!("主动谨慎")));
        assert!(parsed.evidence_summary.contains("行为11 / 第3章"));
    }

    #[test]
    fn parse_implicit_update_decision_rejects_modify_without_value() {
        let err = parse_implicit_update_decision(
            r#"{"decision":"modify","reasoning":"x","new_value":null,"supporting_behavior_ids":[11]}"#,
            &stats(),
        )
        .expect_err("modify without new value should fail");

        assert!(err.contains("new_value"));
    }

    #[test]
    fn parse_implicit_update_decision_rejects_legacy_evidence_summary() {
        let err = parse_implicit_update_decision(
            r#"{"decision":"keep","reasoning":"x","new_value":null,"evidence_summary":"模型自由证据摘要"}"#,
            &stats(),
        )
        .expect_err("legacy evidence_summary must fail");

        assert!(err.contains("evidence_summary 已废弃"));
    }

    #[test]
    fn parse_implicit_update_decision_rejects_unknown_field() {
        let err = parse_implicit_update_decision(
            r#"{"decision":"keep","reasoning":"x","new_value":null,"supporting_behavior_ids":[11],"notes":"多余说明"}"#,
            &stats(),
        )
        .expect_err("unknown implicit update field should fail");

        assert!(err.contains("未支持字段 `notes`"));
    }

    #[test]
    fn parse_implicit_update_decision_rejects_unknown_supporting_behavior_id() {
        let err = parse_implicit_update_decision(
            r#"{"decision":"keep","reasoning":"x","new_value":null,"supporting_behavior_ids":[99]}"#,
            &stats(),
        )
        .expect_err("unknown supporting behavior id must fail");

        assert!(err.contains("不在输入 evidence 候选中"));
    }
}
