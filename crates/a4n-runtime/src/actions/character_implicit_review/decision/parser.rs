use super::super::types::{ImplicitAttributeReviewSummary, ParsedImplicitDecision};
use super::evidence::{materialize_evidence_summary, required_supporting_behavior_ids};
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
