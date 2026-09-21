use std::collections::BTreeSet;

use serde_json::Value;

use crate::character::{
    category_attributes, required_category, required_i32, required_string, required_value,
};

use super::contracts::{
    BehaviorForReview, ImplicitAttributeReviewSummary, ImplicitEvidenceSummary,
    ImplicitReviewSummary,
};

pub(super) fn build_implicit_review_summary(
    current_state: &Value,
    records: &[BehaviorForReview],
    threshold: usize,
    window_chapters: usize,
    evidence_limit: usize,
) -> Result<ImplicitReviewSummary, String> {
    let implicit_attributes = category_attributes(current_state, "implicit")?;
    let implicit_attributes = implicit_attributes
        .as_object()
        .ok_or_else(|| "角色当前状态 implicit_attributes 缺少或不是 JSON 对象".to_string())?;
    let mut attributes = implicit_attributes
        .iter()
        .map(|(key, value)| {
            build_implicit_attribute_review(
                key,
                value.clone(),
                records,
                threshold,
                window_chapters,
                evidence_limit,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    attributes.sort_by(|left, right| {
        right
            .threshold_reached
            .cmp(&left.threshold_reached)
            .then_with(|| right.non_conform_count.cmp(&left.non_conform_count))
            .then_with(|| left.attribute_key.cmp(&right.attribute_key))
    });
    Ok(ImplicitReviewSummary { attributes })
}

pub(super) fn build_implicit_attribute_review(
    attribute_key: &str,
    current_value: Value,
    records: &[BehaviorForReview],
    threshold: usize,
    window_chapters: usize,
    evidence_limit: usize,
) -> Result<ImplicitAttributeReviewSummary, String> {
    let mut total = 0usize;
    let mut conform_count = 0usize;
    let mut non_conform_count = 0usize;
    let mut evidence = Vec::new();
    let mut chapters_seen = BTreeSet::new();

    for record in records {
        if chapters_seen.len() >= window_chapters {
            break;
        }
        let Some(classification) = record
            .classification_results
            .as_object()
            .and_then(|items| items.get(attribute_key))
        else {
            continue;
        };
        chapters_seen.insert(record.chapter_number);
        let normalized = normalized_classification(classification).ok_or_else(|| {
            format!(
                "行为记录 {} 的 classification_results.{attribute_key} 必须是 conform/non-conform",
                record.behavior_id
            )
        })?;
        if evidence.len() < evidence_limit {
            evidence.push(ImplicitEvidenceSummary {
                behavior_id: record.behavior_id,
                chapter_number: record.chapter_number,
                behavior_type: record.behavior_type.clone(),
                behavior_description: record.behavior_description.clone(),
                original_text: record.original_text.clone(),
                behavior_tags: record.behavior_tags.clone(),
                classification: classification.clone(),
            });
        }
        total += 1;
        match normalized {
            "conform" => conform_count += 1,
            "non-conform" => non_conform_count += 1,
            _ => unreachable!("classification was normalized above"),
        }
    }

    let conform_rate = if total == 0 {
        1.0
    } else {
        conform_count as f64 / total as f64
    };
    Ok(ImplicitAttributeReviewSummary {
        attribute_key: attribute_key.to_string(),
        current_value,
        conform_count,
        non_conform_count,
        conform_rate,
        threshold_reached: non_conform_count >= threshold,
        evidence,
    })
}

pub(super) fn behavior_for_review_from_sync(
    index: usize,
    behavior: &Value,
) -> Result<BehaviorForReview, String> {
    let index =
        i64::try_from(index).map_err(|_| format!("隐性属性复审行为索引超过 i64 范围: {index}"))?;
    let behavior_id = index
        .checked_add(1)
        .and_then(i64::checked_neg)
        .ok_or_else(|| format!("隐性属性复审行为索引无法生成临时 ID: {index}"))?;
    Ok(BehaviorForReview {
        behavior_id,
        chapter_number: required_i32(behavior, "chapter_number")?,
        behavior_type: required_string(behavior, "behavior_type")?,
        behavior_description: required_string(behavior, "behavior_description")?,
        original_text: required_string(behavior, "original_text")?,
        behavior_tags: required_value(behavior, "behavior_tags")?.clone(),
        classification_results: required_value(behavior, "classification_results")?.clone(),
    })
}

pub(super) fn implicit_deletion_candidate_keys(
    deletion_candidates: &[Value],
) -> Result<BTreeSet<String>, String> {
    let mut keys = BTreeSet::new();
    for candidate in deletion_candidates {
        if required_category(candidate, "category")? == "implicit" {
            keys.insert(required_string(candidate, "key")?);
        }
    }
    Ok(keys)
}

fn normalized_classification(value: &Value) -> Option<&str> {
    match value.as_str()?.trim() {
        "conform" => Some("conform"),
        "non-conform" => Some("non-conform"),
        _ => None,
    }
}
