use serde_json::json;

#[cfg(test)]
use crate::actions::character_implicit_review::types::ImplicitAttributeReviewSummary;
use crate::actions::llm_request::PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT;

#[cfg(test)]
use super::context::implicit_update_stable_context;
use super::payload::{build_implicit_update_prompt_from_payload, implicit_update_payload};

pub(crate) fn build_implicit_update_manifest_user_prompt(
    profile_name: &str,
    attribute_key: &str,
) -> Result<String, String> {
    let payload = implicit_update_payload(
        profile_name,
        attribute_key,
        json!({
            "context_refs": {
                "attribute_key": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.attribute_key"),
                "current_value": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.current_value"),
                "review_window": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.review_window"),
                "statistics": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.statistics"),
                "evidence": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.evidence")
            }
        }),
    );
    build_implicit_update_prompt_from_payload(payload)
}

#[cfg(test)]
pub(in crate::actions::character_implicit_review::prompt) fn build_implicit_update_user_prompt(
    profile_name: &str,
    attribute_key: &str,
    current_value: &serde_json::Value,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> Result<String, String> {
    let payload = implicit_update_payload(
        profile_name,
        attribute_key,
        implicit_update_stable_context(
            profile_name,
            attribute_key,
            current_value,
            stats,
            threshold,
            window_chapters,
        ),
    );
    build_implicit_update_prompt_from_payload(payload)
}
