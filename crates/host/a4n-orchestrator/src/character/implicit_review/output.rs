use std::collections::{BTreeMap, BTreeSet};

use a4n_domain::character::ImplicitUpdateDecision;
use a4n_domain::json_contract::reject_unknown_fields;
use serde_json::{Map, Value, json};

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

use super::contracts::{
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary, ParsedImplicitDecision,
};

pub(super) fn parse_implicit_update_decision(
    content: &str,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<ParsedImplicitDecision, String> {
    let value: Value = serde_json::from_str(content.trim())
        .map_err(|error| format!("隐性属性复审结果必须是 JSON 对象: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "隐性属性复审结果必须是 JSON 对象".to_string())?;
    let decision = match required_json_string(object, "decision")?.as_str() {
        "keep" => ImplicitUpdateDecision::Keep,
        "modify" => ImplicitUpdateDecision::Modify,
        "delete" => ImplicitUpdateDecision::Delete,
        _ => return Err("隐性属性复审 decision 必须是 keep/modify/delete".to_string()),
    };
    let reasoning = required_json_string(object, "reasoning")?;
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
    if matches!(
        decision,
        ImplicitUpdateDecision::Keep | ImplicitUpdateDecision::Delete
    ) && !new_value.is_null()
    {
        return Err("keep/delete 决策的 new_value 必须为 null".to_string());
    }
    if decision == ImplicitUpdateDecision::Modify && new_value.is_null() {
        return Err("modify 决策的 new_value 不能为空".to_string());
    }
    Ok(ParsedImplicitDecision {
        decision,
        reasoning,
        suggested_new_value: (!new_value.is_null()).then(|| new_value.clone()),
        evidence_summary,
    })
}

pub(super) fn implicit_update_decision_output_json(parsed: &ParsedImplicitDecision) -> Value {
    json!({
        "decision": parsed.decision.as_str(),
        "reasoning": parsed.reasoning,
        "suggested_new_value": parsed.suggested_new_value,
        "evidence_summary": parsed.evidence_summary,
    })
}

pub(super) fn implicit_update_review_checkpoint_response(
    parsed_output: &Value,
    response_text: &str,
    call_key: &str,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<ParsedImplicitDecision, String> {
    let parsed = parse_implicit_update_decision(response_text, stats)?;
    let reparsed_output = implicit_update_decision_output_json(&parsed);
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &reparsed_output,
        &format!("隐性属性复审 {call_key}"),
    )
    .map_err(|error| error.to_string())?;
    Ok(parsed)
}

fn required_supporting_behavior_ids(
    object: &Map<String, Value>,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<Vec<i64>, String> {
    let values = object
        .get("supporting_behavior_ids")
        .and_then(Value::as_array)
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
            let classification = item.classification.as_str().ok_or_else(|| {
                format!(
                    "隐性属性复审行为 {} 的 classification 必须是字符串",
                    item.behavior_id
                )
            })?;
            Ok(format!(
                "行为{} / 第{}章：{}（{}）",
                item.behavior_id,
                item.chapter_number,
                item.behavior_description.trim(),
                classification
            ))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|items| items.join("；"))
}

fn required_json_string(object: &Map<String, Value>, key: &str) -> Result<String, String> {
    let text = object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("隐性属性复审字段 {key} 必须是字符串"))?
        .trim();
    if text.is_empty() {
        return Err(format!("隐性属性复审字段 {key} 不能为空"));
    }
    Ok(text.to_string())
}
