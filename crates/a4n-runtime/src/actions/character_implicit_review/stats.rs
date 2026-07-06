use super::types::{
    BehaviorForReview, ImplicitAttributeReviewSummary, ImplicitEvidenceSummary,
    ImplicitReviewSummary, MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT, MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS,
};

pub(crate) fn validate_implicit_review_params(
    threshold: usize,
    window_chapters: usize,
    evidence_limit: usize,
) -> Result<(usize, usize, usize), String> {
    if threshold == 0 {
        return Err("隐性属性复审 threshold 必须大于 0".to_string());
    }
    if !(1..=MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS).contains(&window_chapters) {
        return Err(format!(
            "隐性属性复审 window_chapters 必须在 1..={MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS} 之间"
        ));
    }
    if !(1..=MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT).contains(&evidence_limit) {
        return Err(format!(
            "隐性属性复审 evidence_limit 必须在 1..={MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT} 之间"
        ));
    }

    Ok((threshold, window_chapters, evidence_limit))
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn behavior(
        id: i64,
        chapter_number: i32,
        classification: serde_json::Value,
    ) -> BehaviorForReview {
        BehaviorForReview {
            behavior_id: id,
            chapter_number,
            behavior_type: "decision".to_string(),
            behavior_description: format!("第{chapter_number}章行为"),
            original_text: "原文证据".to_string(),
            behavior_tags: json!(["标签"]),
            classification_results: classification,
            created_at: None,
        }
    }

    #[test]
    fn review_params_reject_zero_or_out_of_range_values() {
        let threshold_err =
            validate_implicit_review_params(0, 10, 10).expect_err("zero threshold should fail");
        assert!(threshold_err.contains("threshold"));

        let window_err = validate_implicit_review_params(5, 51, 10)
            .expect_err("oversized chapter window should fail");
        assert!(window_err.contains("window_chapters"));

        let evidence_err =
            validate_implicit_review_params(5, 10, 0).expect_err("zero evidence limit should fail");
        assert!(evidence_err.contains("evidence_limit"));
    }

    #[test]
    fn review_rejects_invalid_classification_values() {
        let records = vec![
            behavior(1, 4, json!({"谨慎": "non-conform"})),
            behavior(2, 3, json!({"谨慎": "conform"})),
            behavior(3, 2, json!({"谨慎": true})),
        ];

        let err = build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 1, 10, 10)
            .expect_err("invalid classification should fail");

        assert!(err.contains("classification_results.谨慎"));
    }

    #[test]
    fn review_respects_chapter_window() {
        let records = vec![
            behavior(1, 4, json!({"谨慎": "non-conform"})),
            behavior(2, 3, json!({"谨慎": "non-conform"})),
            behavior(3, 2, json!({"谨慎": "non-conform"})),
            behavior(4, 1, json!({"谨慎": "non-conform"})),
        ];

        let stats =
            build_implicit_attribute_review("谨慎", json!("遇事先观察"), &records, 3, 2, 10)
                .expect("valid records should build review");

        assert_eq!(stats.total, 2);
        assert_eq!(stats.non_conform_count, 2);
        assert!(!stats.threshold_reached);
    }

    #[test]
    fn review_summary_lists_threshold_reached_attributes() {
        let state = json!({
            "implicit_attributes": {
                "谨慎": "遇事先观察",
                "乐观": "相信事情会变好"
            }
        });
        let records = vec![
            behavior(1, 3, json!({"谨慎": "non-conform", "乐观": "conform"})),
            behavior(2, 2, json!({"谨慎": "non-conform", "乐观": "conform"})),
        ];

        let review = build_implicit_review_summary(7, &state, &records, 2, 10, 10)
            .expect("canonical state should build review");

        assert_eq!(review.needs_review, vec!["谨慎"]);
        assert_eq!(review.attributes[0].attribute_key, "谨慎");
    }

    #[test]
    fn review_summary_rejects_missing_implicit_state() {
        let err = build_implicit_review_summary(7, &json!({}), &[], 2, 10, 10)
            .expect_err("missing implicit state should fail");

        assert!(err.contains("implicit_attributes"));
    }
}
