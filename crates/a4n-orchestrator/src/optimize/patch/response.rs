use a4n_core::error::AppError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use super::contracts::{OptimizationPatch, OptimizationPatchProposal};
use super::validation::{normalize_optimization_patch, validate_optimization_patches};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OptimizationPatchLlmResponse {
    summary: String,
    patches: Vec<OptimizationPatch>,
}

pub(in crate::optimize) fn parse_content_optimization_patch_response(
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
    base_content_hash: &str,
    chapter_content: &str,
    raw: &str,
) -> Result<OptimizationPatchProposal, AppError> {
    let value: Value = serde_json::from_str(raw.trim()).map_err(|e| {
        AppError::InvalidArgument(format!(
            "content_optimization_patch 必须返回严格 JSON 对象: {e}"
        ))
    })?;
    parse_content_optimization_patch_value(
        session_id,
        project_id,
        chapter_id,
        base_content_hash,
        chapter_content,
        &value,
    )
}

pub(in crate::optimize) fn content_optimization_patch_schema() -> Result<Value, AppError> {
    serde_json::to_value(schemars::schema_for!(OptimizationPatchLlmResponse))
        .map_err(AppError::Serialization)
}

pub(in crate::optimize) fn parse_content_optimization_patch_value(
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
    base_content_hash: &str,
    chapter_content: &str,
    value: &Value,
) -> Result<OptimizationPatchProposal, AppError> {
    let parsed: OptimizationPatchLlmResponse =
        serde_json::from_value(value.clone()).map_err(|e| {
            AppError::InvalidArgument(format!(
                "content_optimization_patch 必须返回严格 JSON 对象: {e}"
            ))
        })?;
    let summary = parsed.summary.trim();
    if summary.is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization_patch.summary 不能为空".to_string(),
        ));
    }
    if !parsed.patches.is_empty() {
        validate_optimization_patches(chapter_content, &parsed.patches)?;
    }
    Ok(OptimizationPatchProposal {
        session_id,
        project_id,
        chapter_id,
        base_content_hash: base_content_hash.to_string(),
        summary: summary.to_string(),
        patches: parsed
            .patches
            .into_iter()
            .map(normalize_optimization_patch)
            .collect(),
    })
}

pub(in crate::optimize) fn restore_content_optimization_patch_proposal(
    value: Value,
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
    base_content_hash: &str,
    chapter_content: &str,
) -> Result<OptimizationPatchProposal, AppError> {
    let proposal: OptimizationPatchProposal = serde_json::from_value(value)?;
    if proposal.session_id != session_id
        || proposal.project_id != project_id
        || proposal.chapter_id != chapter_id
        || proposal.base_content_hash != base_content_hash
    {
        return Err(AppError::InvalidState {
            current: "optimization_patch_checkpoint_identity_mismatch".to_string(),
            operation: "恢复内容优化局部补丁断点".to_string(),
        });
    }
    if proposal.summary.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization_patch.summary 不能为空".to_string(),
        ));
    }
    validate_optimization_patches(chapter_content, &proposal.patches)?;
    Ok(proposal)
}
