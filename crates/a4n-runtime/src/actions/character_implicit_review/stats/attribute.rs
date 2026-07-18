use super::super::types::{
    BehaviorForReview, ImplicitAttributeReviewSummary, ImplicitEvidenceSummary,
};

fn classification_value<'a>(
    classification_results: &'a serde_json::Value,
    attribute_key: &str,
) -> Option<&'a serde_json::Value> {
    classification_results
        .as_object()
        .and_then(|items| items.get(attribute_key))
}

fn normalized_classification(value: &serde_json::Value) -> Option<&str> {
    let text = value.as_str()?.trim();
    match text {
        "conform" => Some("conform"),
        "non-conform" => Some("non-conform"),
        _ => None,
    }
}

fn implicit_evidence_from_behavior(
    behavior: &BehaviorForReview,
    classification: serde_json::Value,
) -> ImplicitEvidenceSummary {
    ImplicitEvidenceSummary {
        behavior_id: behavior.behavior_id,
        chapter_number: behavior.chapter_number,
        behavior_type: behavior.behavior_type.clone(),
        behavior_description: behavior.behavior_description.clone(),
        original_text: behavior.original_text.clone(),
        behavior_tags: behavior.behavior_tags.clone(),
        classification,
        created_at: behavior.created_at.clone(),
    }
}

pub(crate) fn build_implicit_attribute_review(
    attribute_key: &str,
    current_value: serde_json::Value,
    records: &[BehaviorForReview],
    threshold: usize,
    window_chapters: usize,
    evidence_limit: usize,
) -> Result<ImplicitAttributeReviewSummary, String> {
    let mut total = 0usize;
    let mut conform_count = 0usize;
    let mut non_conform_count = 0usize;
    let mut evidence = Vec::new();
    let mut chapters_seen = std::collections::BTreeSet::new();

    for record in records {
        if chapters_seen.len() >= window_chapters {
            break;
        }
        let Some(classification) =
            classification_value(&record.classification_results, attribute_key)
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
            evidence.push(implicit_evidence_from_behavior(
                record,
                classification.clone(),
            ));
        }

        match normalized {
            "conform" => {
                total += 1;
                conform_count += 1;
            }
            "non-conform" => {
                total += 1;
                non_conform_count += 1;
            }
            _ => unreachable!("normalized classification is already validated"),
        }
    }

    let conform_rate = if total > 0 {
        conform_count as f64 / total as f64
    } else {
        1.0
    };

    Ok(ImplicitAttributeReviewSummary {
        attribute_key: attribute_key.to_string(),
        current_value,
        total,
        conform_count,
        non_conform_count,
        conform_rate,
        threshold_reached: non_conform_count >= threshold,
        evidence,
    })
}
