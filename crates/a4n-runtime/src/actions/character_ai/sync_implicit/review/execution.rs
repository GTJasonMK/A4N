use crate::actions::character_implicit_review::{
    AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
    AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, ImplicitUpdateDecisionSummary,
    build_implicit_review_summary, validate_implicit_review_params,
};
use crate::state::AppState;
use a4n_core::models::character::protagonist_profile;
use serde_json::Value;

use super::inputs::{behavior_records_for_review, implicit_deletion_candidate_keys};

mod call;

pub(in crate::actions::character_ai) async fn decide_implicit_reviews(
    state: &AppState,
    workflow_run_id: Option<i64>,
    profile: &protagonist_profile::Model,
    start_chapter: i32,
    final_attributes: &Value,
    behaviors: &[Value],
    deletion_candidates: &[Value],
) -> Result<Vec<ImplicitUpdateDecisionSummary>, String> {
    let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
        AUTO_IMPLICIT_REVIEW_THRESHOLD,
        AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS,
        AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT,
    )?;
    let records = behavior_records_for_review(state, profile.id, start_chapter, behaviors).await?;
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let excluded_keys = implicit_deletion_candidate_keys(deletion_candidates)?;
    let review = build_implicit_review_summary(
        profile.id,
        final_attributes,
        &records,
        threshold,
        window_chapters,
        evidence_limit,
    )?;
    let mut decisions = Vec::new();
    for stats in review
        .attributes
        .into_iter()
        .filter(|stats| stats.threshold_reached && !excluded_keys.contains(&stats.attribute_key))
    {
        let call_index = decisions.len();
        decisions.push(
            call::run_implicit_review_call(
                state,
                profile,
                stats,
                call::ImplicitReviewCallInput {
                    workflow_run_id,
                    start_chapter,
                    threshold,
                    window_chapters,
                    evidence_limit,
                    call_index,
                },
            )
            .await?,
        );
    }

    Ok(decisions)
}
