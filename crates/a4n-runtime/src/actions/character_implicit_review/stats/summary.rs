use super::super::types::{
    BehaviorForReview, ImplicitAttributeReviewSummary, ImplicitReviewSummary,
};
use super::attribute::build_implicit_attribute_review;

fn required_category_attributes<'a>(
    value: &'a serde_json::Value,
    category: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, String> {
    let key = match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        other => other,
    };
    value
        .get(key)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| format!("角色当前状态 {key} 缺少或不是 JSON 对象"))
}

pub(crate) fn build_implicit_review_summary(
    profile_id: i64,
    current_state: &serde_json::Value,
    records: &[BehaviorForReview],
    threshold: usize,
    window_chapters: usize,
    evidence_limit: usize,
) -> Result<ImplicitReviewSummary, String> {
    let implicit_attributes = required_category_attributes(current_state, "implicit")?.clone();

    let mut attributes: Vec<ImplicitAttributeReviewSummary> = implicit_attributes
        .into_iter()
        .map(|(key, value)| {
            build_implicit_attribute_review(
                &key,
                value,
                records,
                threshold,
                window_chapters,
                evidence_limit,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    attributes.sort_by(|a, b| {
        b.threshold_reached
            .cmp(&a.threshold_reached)
            .then_with(|| b.non_conform_count.cmp(&a.non_conform_count))
            .then_with(|| a.attribute_key.cmp(&b.attribute_key))
    });

    let needs_review = attributes
        .iter()
        .filter(|item| item.threshold_reached)
        .map(|item| item.attribute_key.clone())
        .collect();

    Ok(ImplicitReviewSummary {
        profile_id,
        threshold,
        window_chapters,
        evidence_limit,
        attributes,
        needs_review,
    })
}
