use super::{
    GetImplicitReviewInput, ImplicitReviewSummary, build_implicit_review_summary,
    context::implicit_review_context_for_user, validate_implicit_review_params,
};
use crate::AppState;

mod execution;
mod preview;

pub use execution::check_implicit_update;
pub use preview::preview_check_implicit_update_request;

pub async fn get_implicit_review(
    state: &AppState,
    user_id: i64,
    input: GetImplicitReviewInput,
) -> Result<ImplicitReviewSummary, String> {
    let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
        input.threshold,
        input.window_chapters,
        input.evidence_limit,
    )?;
    let (profile, records) =
        implicit_review_context_for_user(state, user_id, input.profile_id).await?;
    build_implicit_review_summary(
        input.profile_id,
        &profile.current_state,
        &records,
        threshold,
        window_chapters,
        evidence_limit,
    )
}
