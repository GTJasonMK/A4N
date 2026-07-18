use std::collections::{BTreeMap, BTreeSet};

use super::super::types::{ImplicitAttributeReviewSummary, ImplicitEvidenceSummary};

pub(super) fn required_supporting_behavior_ids(
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

pub(super) fn materialize_evidence_summary(
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

pub(super) fn format_evidence_item(item: &ImplicitEvidenceSummary) -> String {
    let classification = item.classification.as_str().unwrap_or("unknown");
    format!(
        "行为{} / 第{}章：{}（{}）",
        item.behavior_id,
        item.chapter_number,
        item.behavior_description.trim(),
        classification
    )
}
